use std::{
    fs::FileType,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};
use pathdiff::diff_paths;
use tokio::fs;

pub(crate) trait PathExt {
    fn base(&self) -> Result<&Self>;

    async fn file_type(&self) -> Result<FileType>;

    async fn is_dir_nofollow(&self) -> Result<bool>;

    async fn is_file_nofollow(&self) -> Result<bool>;

    async fn is_symlink_nofollow(&self) -> Result<bool>;

    async fn create_relative_symlink_atomically_at(
        &self,
        symlink_path: impl AsRef<Self>,
    ) -> Result<PathBuf>;
}

impl PathExt for Path {
    fn base(&self) -> Result<&Self> {
        let base_path = self.parent().context("No parent directory found")?;

        Ok(base_path)
    }

    async fn file_type(&self) -> Result<FileType> {
        let metadata = fs::symlink_metadata(self).await?;

        let file_type = metadata.file_type();

        Ok(file_type)
    }

    async fn is_dir_nofollow(&self) -> Result<bool> {
        let file_type = self.file_type().await?;

        let is_dir = file_type.is_dir();

        Ok(is_dir)
    }

    async fn is_file_nofollow(&self) -> Result<bool> {
        let file_type = self.file_type().await?;

        let is_file = file_type.is_file();

        Ok(is_file)
    }

    async fn is_symlink_nofollow(&self) -> Result<bool> {
        let file_type = self.file_type().await?;

        let is_symlink = file_type.is_symlink();

        Ok(is_symlink)
    }

    async fn create_relative_symlink_atomically_at(
        &self,
        symlink_path: impl AsRef<Self>,
    ) -> Result<PathBuf> {
        let symlink_path = symlink_path.as_ref();

        let symlink_base_path = symlink_path.base()?;

        let symlink_diff_path =
            diff_paths(self, symlink_base_path).context("Failed to diff paths")?;

        let symlink_tmp_path = symlink_path.with_added_extension("tmp");

        fs::symlink(symlink_diff_path, &symlink_tmp_path).await?;

        fs::rename(symlink_tmp_path, symlink_path).await?;

        let symlink_path = symlink_path.to_owned();

        Ok(symlink_path)
    }
}
