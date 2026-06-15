use std::{borrow::Cow, collections::HashMap, path::Path};

use arwen::elf::rewriter::Writer;
use bytes::Bytes;
use tempfile::NamedTempFile;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt as _,
    task,
};
use tokio_util::task::AbortOnDropHandle;

use super::{Relocator, RelocatorExt, ReplacementPairs};
use crate::{
    ext::{std::path::PathExt as _, tokio::fs::FileExt as _},
    util::linux::elf::Elf,
};

impl RelocatorExt for Relocator {
    async fn patch_file(
        &self,
        dest_file_path: &Path,
        replacement_pairs: &ReplacementPairs,
    ) -> anyhow::Result<()> {
        let has_magic = Elf::has_magic(dest_file_path).await?;

        if !has_magic {
            return Ok(());
        }

        let bytes = fs::read(dest_file_path).await?;
        let bytes = Bytes::from(bytes);

        let this = self.clone();

        let handle = task::spawn_blocking({
            let bytes = bytes.clone();

            let replacement_pairs = replacement_pairs.clone();

            move || {
                let replaced_bytes = this.replace_bytes(&bytes, &replacement_pairs)?;

                anyhow::Ok(replaced_bytes)
            }
        });
        let handle = AbortOnDropHandle::new(handle);

        let replaced_bytes = handle.await??;

        if replaced_bytes == *bytes {
            return Ok(());
        }

        let metadata = fs::symlink_metadata(dest_file_path).await?;

        let permissions = metadata.permissions();

        let dest_file_base_path = dest_file_path.base()?;

        let temp_file = NamedTempFile::new_in(dest_file_base_path)?;

        let temp_file_path = temp_file.path();

        let mut async_temp_file = File::open_write(temp_file_path).await?;

        async_temp_file.write_all(&replaced_bytes).await?;

        async_temp_file.shutdown().await?;

        let dest_file = temp_file.persist(dest_file_path)?;

        dest_file.set_permissions(permissions)?;

        Ok(())
    }

    fn replace_bytes(
        &self,
        bytes: &Bytes,
        replacement_pairs: &ReplacementPairs,
    ) -> anyhow::Result<Vec<u8>> {
        let mut rewriter = Writer::read(bytes)?;

        if let Some(runpath) = rewriter.elf_runpath() {
            let old_runpath = String::from_utf8_lossy(runpath);

            let new_runpath = old_runpath
                .split(':')
                .map(|component| self.replace_pstr(component, replacement_pairs))
                .collect::<Vec<_>>();
            let new_runpath = new_runpath.join(":");

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
                let new_need = self.replace_pstr(&old_need, replacement_pairs);

                match new_need {
                    Cow::Owned(new_string) => {
                        let old_need = old_need.into_owned();
                        let old_need = old_need.into_bytes();

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

        let mut replaced_bytes = Vec::new();

        rewriter.write(&mut replaced_bytes)?;

        Ok(replaced_bytes)
    }
}
