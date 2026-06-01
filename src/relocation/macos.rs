use std::{path::Path, sync::Arc};

use arwen::macho::{MachoContainer, MachoType};
use tempfile::NamedTempFile;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt as _,
    task,
};
use tokio_util::task::AbortOnDropHandle;

use super::{Relocator, RelocatorInner};
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::fs::FileExt as _},
    util::macos,
};

#[derive(Clone)]
pub(crate) struct Relocation {
    replacement_pairs: [(&'static str, String); 4],

    context: Arc<Context>,
}

impl From<([(&'static str, String); 4], Arc<Context>)> for Relocation {
    fn from((replacement_pairs, context): ([(&'static str, String); 4], Arc<Context>)) -> Self {
        Self {
            replacement_pairs,

            context,
        }
    }
}

impl Relocator for Relocation {}

impl RelocatorInner for Relocation {
    fn replacement_pairs(&self) -> &[(&'static str, String); 4] {
        &self.replacement_pairs
    }

    fn context(&self) -> &Context {
        &self.context
    }

    async fn patch_file(&self, path: &Path) -> anyhow::Result<()> {
        let bytes = fs::read(path).await?;
        let bytes = Arc::from(bytes);

        let has_magic = macos::MachO::has_magic(&bytes);

        if !has_magic {
            return Ok(());
        }

        let cloned_self = self.clone();

        let cloned_bytes = Arc::clone(&bytes);

        let handle = task::spawn_blocking(move || {
            let replaced_bytes = cloned_self.replace_bytes(&cloned_bytes)?;

            anyhow::Ok(replaced_bytes)
        });
        let handle = AbortOnDropHandle::new(handle);

        let replaced_bytes = handle.await??;

        if replaced_bytes == *bytes {
            return Ok(());
        }

        let metadata = fs::symlink_metadata(path).await?;

        let permissions = metadata.permissions();

        let base_path = path.base()?;

        let temp_file = NamedTempFile::new_in(base_path)?;

        let mut async_temp_file = File::open_write(temp_file.path()).await?;

        async_temp_file.write_all(&replaced_bytes).await?;

        async_temp_file.shutdown().await?;

        let file = temp_file.persist(path)?;

        file.set_permissions(permissions)?;

        macos::Codesign::in_place(path).await?;

        Ok(())
    }

    fn replace_bytes(&self, bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut container = MachoContainer::parse(bytes)?;

        let rpaths = match &container.inner {
            MachoType::SingleArch(single) => single.inner.rpaths.clone(),
            MachoType::Fat(fat) => fat
                .archs
                .iter()
                .flat_map(|arch| arch.inner.inner.rpaths.iter().copied())
                .collect::<Vec<_>>(),
        };

        let install_ids = match &container.inner {
            MachoType::SingleArch(single) => single.inner.name.into_iter().collect::<Vec<_>>(),
            MachoType::Fat(fat) => fat
                .archs
                .iter()
                .filter_map(|arch| arch.inner.inner.name)
                .collect::<Vec<_>>(),
        };

        let install_names = match &container.inner {
            MachoType::SingleArch(single) => single.inner.libs.clone(),
            MachoType::Fat(fat) => fat
                .archs
                .iter()
                .flat_map(|arch| arch.inner.inner.libs.iter().copied())
                .collect::<Vec<_>>(),
        };

        for old_rpath in rpaths {
            let new_rpath = self.replace_text(old_rpath);

            if new_rpath != old_rpath {
                container.change_rpath(old_rpath, &new_rpath)?;
            }
        }

        for old_install_id in install_ids {
            let new_install_id = self.replace_text(old_install_id);

            if new_install_id != old_install_id {
                container.change_install_id(&new_install_id)?;
            }
        }

        for old_install_name in install_names {
            let new_install_name = self.replace_text(old_install_name);

            if new_install_name != old_install_name {
                container.change_install_name(old_install_name, &new_install_name)?;
            }
        }

        let data = container.data;

        Ok(data)
    }
}
