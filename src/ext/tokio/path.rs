use std::{
    fs::FileType,
    io::{self, ErrorKind},
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
};

use pathdiff::diff_paths;
use tokio::fs;

use super::super::std::path::PathExt as _;

pub(crate) trait PathExt {
    async fn realpath(&self) -> io::Result<PathBuf>;

    async fn realpath_or_none(&self) -> io::Result<Option<PathBuf>>;

    async fn file_type(&self) -> io::Result<FileType>;

    async fn is_dir_empty(&self) -> io::Result<bool>;

    async fn add_permissions_mode(&self, mode: u32) -> io::Result<()>;

    async fn is_dir_exists_nofollow(&self) -> io::Result<bool>;

    async fn is_file_exists_nofollow(&self) -> io::Result<bool>;

    async fn is_symlink_exists_nofollow(&self) -> io::Result<bool>;

    async fn create_relative_symlink_atomically_at(
        &self,
        symlink_path: impl AsRef<Self>,
    ) -> io::Result<PathBuf>;
}

impl PathExt for Path {
    async fn realpath(&self) -> io::Result<PathBuf> {
        let path = fs::canonicalize(self).await?;

        Ok(path)
    }

    async fn realpath_or_none(&self) -> io::Result<Option<PathBuf>> {
        let path = match fs::canonicalize(self).await {
            Ok(path) => path,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };

        Ok(Some(path))
    }

    async fn file_type(&self) -> io::Result<FileType> {
        let metadata = fs::symlink_metadata(self).await?;

        let file_type = metadata.file_type();

        Ok(file_type)
    }

    async fn is_dir_empty(&self) -> io::Result<bool> {
        let mut entries = match fs::read_dir(self).await {
            Ok(entries) => entries,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(true),
            Err(err) => return Err(err),
        };

        let entry = entries.next_entry().await?;

        let is_dir_empty = entry.is_none();

        Ok(is_dir_empty)
    }

    async fn add_permissions_mode(&self, mode: u32) -> io::Result<()> {
        let metadata = fs::symlink_metadata(self).await?;

        let mut permissions = metadata.permissions();

        permissions.set_mode(permissions.mode() | mode);

        fs::set_permissions(self, permissions).await?;

        Ok(())
    }

    async fn is_dir_exists_nofollow(&self) -> io::Result<bool> {
        let metadata = match fs::symlink_metadata(self).await {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
            Err(err) => return Err(err),
        };

        let file_type = metadata.file_type();

        let is_dir = file_type.is_dir();

        Ok(is_dir)
    }

    async fn is_file_exists_nofollow(&self) -> io::Result<bool> {
        let metadata = match fs::symlink_metadata(self).await {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
            Err(err) => return Err(err),
        };

        let file_type = metadata.file_type();

        let is_file = file_type.is_file();

        Ok(is_file)
    }

    async fn is_symlink_exists_nofollow(&self) -> io::Result<bool> {
        let metadata = match fs::symlink_metadata(self).await {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
            Err(err) => return Err(err),
        };

        let file_type = metadata.file_type();

        let is_symlink = file_type.is_symlink();

        Ok(is_symlink)
    }

    async fn create_relative_symlink_atomically_at(
        &self,
        symlink_path: impl AsRef<Self>,
    ) -> io::Result<PathBuf> {
        let symlink_path = symlink_path.as_ref();

        let symlink_base_path = symlink_path.base();
        let symlink_base_path = symlink_base_path.map_err(io::Error::other)?;

        let symlink_diff_path = diff_paths(self, symlink_base_path)
            .ok_or_else(|| io::Error::other("Failed to diff paths"))?;

        let symlink_tmp_path = symlink_path.with_added_extension("tmp");

        fs::symlink(symlink_diff_path, &symlink_tmp_path).await?;

        fs::rename(symlink_tmp_path, symlink_path).await?;

        let symlink_path = symlink_path.to_owned();

        Ok(symlink_path)
    }
}
