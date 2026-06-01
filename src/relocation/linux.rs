use std::{borrow::Cow, collections::HashMap, path::Path, sync::Arc};

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

        let has_magic = linux::Elf::has_magic(&bytes);

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
                let new_need = self.replace_text(&old_need);

                match new_need {
                    Cow::Owned(new_string) => {
                        let old_need = old_need.into_owned().into_bytes();

                        let new_need = new_string.into_bytes();

                        Some((old_need, new_need))
                    },
                    Cow::Borrowed(_) => None,
                }
            })
            .collect::<HashMap<_, _>>();

        if !new_needed.is_empty() {
            rewriter.elf_replace_needed(&new_needed)?;
        }

        let mut data = Vec::new();

        rewriter.write(&mut data)?;

        Ok(data)
    }
}
