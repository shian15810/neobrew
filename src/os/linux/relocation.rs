use std::{cmp::Reverse, collections::HashMap, fs, io::Write as _, path::Path};

use anyhow::{Context as _, Result};
use arwen::elf::rewriter::Writer;
use tempfile::NamedTempFile;

use super::elf::Elf;
use crate::context::dirs::HomebrewDirs;

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
    pub(crate) fn patch_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        let bytes = fs::read(path)?;

        let has_magic_number = Elf::has_magic_number(&bytes)?;

        if !has_magic_number {
            return Ok(());
        }

        let replaced_bytes = self.replace_bytes(&bytes)?;

        if replaced_bytes == bytes {
            return Ok(());
        }

        let metadata = fs::metadata(path)?;

        let permissions = metadata.permissions();

        let path_parent = path
            .parent()
            .context("Relocating ELF binary has no parent")?;

        let mut file = NamedTempFile::new_in(path_parent)?;

        file.write_all(&replaced_bytes)?;

        let file = file.persist(path)?;

        file.set_permissions(permissions)?;

        Ok(())
    }

    fn replace_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut rewriter = Writer::read(bytes)?;

        if let Some(runpath) = rewriter.elf_runpath() {
            let old_runpath = String::from_utf8_lossy(runpath);

            let new_runpath = old_runpath
                .split(':')
                .map(|component| self.replace_text(component))
                .collect::<Vec<_>>();
            let new_runpath = new_runpath.join(":");

            if new_runpath != old_runpath {
                let new_runpath = new_runpath.into_bytes();

                rewriter.elf_set_runpath(new_runpath)?;
            }
        }

        let old_needed = rewriter
            .elf_needed()
            .map(|bytes| String::from_utf8_lossy(bytes))
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
