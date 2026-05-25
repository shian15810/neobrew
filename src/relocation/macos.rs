use std::{
    cmp::Reverse,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use arwen::macho::{MachoContainer, MachoType};
use async_walkdir::WalkDir;
use futures::stream::StreamExt as _;
use tempfile::NamedTempFile;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt as _,
    task,
};
use tokio_util::task::AbortOnDropHandle;

use crate::{
    context::dirs::HomebrewDirs,
    ext::{
        std::path::PathExt as _,
        tokio::{fs::FileExt as _, path::PathExt as _},
    },
    util::macos::{Codesign, MachO},
};

pub(crate) struct Relocation {
    replacement_pairs: [(&'static str, String); 4],
}

impl Relocation {
    const PREFIX_PLACEHOLDER: &str = "@@HOMEBREW_PREFIX@@";
    const CELLAR_PLACEHOLDER: &str = "@@HOMEBREW_CELLAR@@";
    const REPOSITORY_PLACEHOLDER: &str = "@@HOMEBREW_REPOSITORY@@";
    const LIBRARY_PLACEHOLDER: &str = "@@HOMEBREW_LIBRARY@@";
}

impl From<&HomebrewDirs> for Relocation {
    fn from(homebrew_dirs: &HomebrewDirs) -> Self {
        let replacement_pairs = [
            (Self::PREFIX_PLACEHOLDER, homebrew_dirs.prefix_dir()),
            (Self::CELLAR_PLACEHOLDER, homebrew_dirs.cellar_dir()),
            (Self::REPOSITORY_PLACEHOLDER, homebrew_dirs.repository_dir()),
            (Self::LIBRARY_PLACEHOLDER, homebrew_dirs.library_dir()),
        ];
        let mut replacement_pairs = replacement_pairs.map(|(placeholder, replacement)| {
            let replacement = replacement.to_string_lossy();
            let replacement = replacement.into_owned();

            (placeholder, replacement)
        });

        replacement_pairs.sort_by_key(|(placeholder, _)| Reverse(placeholder.len()));

        Self {
            replacement_pairs,
        }
    }
}

impl Relocation {
    pub(crate) async fn patch_keg(self: Arc<Self>, keg_dir_path: &Path) -> Result<()> {
        let mut entries = WalkDir::new(keg_dir_path);

        while let Some(entry) = entries.next().await {
            let path = entry?.path();

            if !path.is_file_exists_nofollow().await? {
                continue;
            }

            let this = Arc::clone(&self);

            this.patch_file(path).await?;
        }

        Ok(())
    }

    async fn patch_file(self: Arc<Self>, path: PathBuf) -> Result<()> {
        let bytes = fs::read(&path).await?;

        let has_magic_number = MachO::has_magic_number(&bytes)?;

        if !has_magic_number {
            return Ok(());
        }

        let handle = task::spawn_blocking(move || {
            let replaced_bytes = self.replace_bytes(&bytes)?;

            if replaced_bytes == bytes {
                return Ok(None);
            }

            anyhow::Ok(Some(replaced_bytes))
        });

        let handle = AbortOnDropHandle::new(handle);

        let Some(replaced_bytes) = handle.await?? else {
            return Ok(());
        };

        let metadata = fs::symlink_metadata(&path).await?;

        let permissions = metadata.permissions();

        let base_path = path.base()?;

        let file = NamedTempFile::new_in(base_path)?;

        let mut async_file = File::open_write(file.path()).await?;

        async_file.write_all(&replaced_bytes).await?;

        async_file.shutdown().await?;

        let file = file.persist(&path)?;

        file.set_permissions(permissions)?;

        Codesign::in_place(path).await?;

        Ok(())
    }

    fn replace_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>> {
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

    fn replace_text(&self, text: &str) -> String {
        self.replacement_pairs
            .iter()
            .fold(text.to_owned(), |text, (placeholder, replacement)| {
                text.replace(placeholder, replacement)
            })
    }
}
