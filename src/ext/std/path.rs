use std::{
    fs::{self, FileType},
    os::unix,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};
use pathdiff::diff_paths;

pub(crate) trait PathExt {
    fn base(&self) -> Result<&Self>;

    fn file_type(&self) -> Result<FileType>;

    fn is_dir_nofollow(&self) -> Result<bool>;

    fn is_file_nofollow(&self) -> Result<bool>;

    fn is_symlink_nofollow(&self) -> Result<bool>;

    fn create_relative_symlink_atomically_at(
        &self,
        symlink_path: impl AsRef<Self>,
    ) -> Result<PathBuf>;
}

impl PathExt for Path {
    fn base(&self) -> Result<&Self> {
        let base_path = self.parent().context("No parent directory found")?;

        Ok(base_path)
    }

    fn file_type(&self) -> Result<FileType> {
        let metadata = fs::symlink_metadata(self)?;

        let file_type = metadata.file_type();

        Ok(file_type)
    }

    fn is_dir_nofollow(&self) -> Result<bool> {
        let file_type = self.file_type()?;

        let is_dir = file_type.is_dir();

        Ok(is_dir)
    }

    fn is_file_nofollow(&self) -> Result<bool> {
        let file_type = self.file_type()?;

        let is_file = file_type.is_file();

        Ok(is_file)
    }

    fn is_symlink_nofollow(&self) -> Result<bool> {
        let file_type = self.file_type()?;

        let is_symlink = file_type.is_symlink();

        Ok(is_symlink)
    }

    fn create_relative_symlink_atomically_at(
        &self,
        symlink_path: impl AsRef<Self>,
    ) -> Result<PathBuf> {
        let symlink_path = symlink_path.as_ref();

        let symlink_base_path = symlink_path.base()?;

        let symlink_diff_path =
            diff_paths(self, symlink_base_path).context("Failed to diff paths")?;

        let symlink_tmp_path = symlink_path.with_added_extension("tmp");

        unix::fs::symlink(symlink_diff_path, &symlink_tmp_path)?;

        fs::rename(symlink_tmp_path, symlink_path)?;

        let symlink_path = symlink_path.to_owned();

        Ok(symlink_path)
    }
}
