use std::{collections::HashMap, path::Path, sync::Arc};

use arwen::elf::rewriter::Writer;
use itertools::Itertools as _;
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
    util::linux,
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

        let has_magic_number = linux::Elf::has_magic_number(&bytes);

        if !has_magic_number {
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

        let file = NamedTempFile::new_in(base_path)?;

        let mut async_file = File::open_write(file.path()).await?;

        async_file.write_all(&replaced_bytes).await?;

        async_file.shutdown().await?;

        let file = file.persist(path)?;

        file.set_permissions(permissions)?;

        Ok(())
    }

    fn replace_bytes(&self, bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut rewriter = Writer::read(bytes)?;

        if let Some(runpath) = rewriter.elf_runpath() {
            let old_runpath = String::from_utf8_lossy(runpath);

            let new_runpath = old_runpath
                .split(':')
                .map(|component| self.replace_text(component))
                .join(":");

            if new_runpath != old_runpath {
                let new_runpath = new_runpath.into_bytes();

                rewriter.elf_set_runpath(new_runpath)?;
            }
        }

        let old_needed = rewriter
            .elf_needed()
            .map(String::from_utf8_lossy)
            .collect::<Vec<_>>();

        let new_needed = old_needed
            .into_iter()
            .filter_map(|old_need| {
                let old_need = old_need.into_owned();

                let new_need = self.replace_text(&old_need);

                (new_need != old_need).then(|| {
                    let old_need = old_need.into_bytes();

                    let new_need = new_need.into_bytes();

                    (old_need, new_need)
                })
            })
            .collect::<HashMap<_, _>>();

        if !new_needed.is_empty() {
            rewriter.elf_replace_needed(&new_needed)?;
        }

        let mut data = Vec::new();

        rewriter.write(&mut data)?;

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
