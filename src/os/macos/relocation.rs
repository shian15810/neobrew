use std::{cmp::Reverse, fs, io::Write as _, path::Path};

use anyhow::{Context as _, Result};
use arwen::macho::{MachoContainer, MachoType};
use tempfile::NamedTempFile;

use super::mach_o::MachO;
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
            (placeholder, replacement.to_string_lossy().into_owned())
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

        let Some(_) = MachO::detect_magic_number(&bytes)? else {
            return Ok(());
        };

        let replaced_bytes = self.replace_bytes(&bytes)?;

        if replaced_bytes == bytes {
            return Ok(());
        }

        let metadata = fs::metadata(path)?;

        let permissions = metadata.permissions();

        let path_parent = path.parent().context("Path has no parent")?;

        let mut file = NamedTempFile::new_in(path_parent)?;

        file.write_all(&replaced_bytes)?;

        let file = file.persist(path)?;

        file.set_permissions(permissions)?;

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
            let new_rpath = self.replace_text(old_rpath.to_owned());

            if new_rpath != old_rpath {
                container.change_rpath(old_rpath, &new_rpath)?;
            }
        }

        for old_install_id in install_ids {
            let new_install_id = self.replace_text(old_install_id.to_owned());

            if new_install_id != old_install_id {
                container.change_install_id(&new_install_id)?;
            }
        }

        for old_install_name in install_names {
            let new_install_name = self.replace_text(old_install_name.to_owned());

            if new_install_name != old_install_name {
                container.change_install_name(old_install_name, &new_install_name)?;
            }
        }

        Ok(container.data)
    }

    fn replace_text(&self, text: String) -> String {
        self.replacement_pairs
            .iter()
            .fold(text, |text, (placeholder, replacement)| {
                text.replace(placeholder, replacement)
            })
    }
}
