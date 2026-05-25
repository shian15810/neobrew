use std::path::PathBuf;

use anyhow::Result;
use async_compression::tokio::bufread::GzipDecoder;
use tempfile::TempDir;
use tokio::{
    fs,
    io::{AsyncRead, BufReader},
};
use tokio_tar::Archive;

use super::{super::handler, PullOperator};
use crate::ext::tokio::path::PathExt as _;

pub(crate) struct TempPourer {
    dir_path: PathBuf,
    symlink_paths: Vec<PathBuf>,
}

impl TempPourer {
    pub(crate) fn create(dir_path: PathBuf, symlink_paths: Vec<PathBuf>) -> Self {
        Self {
            dir_path,
            symlink_paths,
        }
    }
}

impl PullOperator for TempPourer {
    type Output = TempPourerOutput;

    async fn from_reader(self, reader: impl AsyncRead + Unpin + Send) -> Result<Self::Output> {
        fs::create_dir_all(&self.dir_path).await?;

        let dir = TempDir::new_in(&self.dir_path)?;

        let buf_reader = BufReader::new(reader);

        let gz_decoder = GzipDecoder::new(buf_reader);

        let mut archive = Archive::new(gz_decoder);

        archive.unpack(dir.path()).await?;

        let output = TempPourerOutput {
            dir,

            dir_path: self.dir_path,
            symlink_paths: self.symlink_paths,
        };

        Ok(output)
    }
}

pub(crate) struct TempPourerOutput {
    dir: TempDir,

    dir_path: PathBuf,
    symlink_paths: Vec<PathBuf>,
}

impl handler::AtomicWriter for TempPourerOutput {
    async fn cleanup(self) -> Result<()> {
        self.dir.close()?;

        Ok(())
    }

    async fn persist(self) -> Result<()> {
        let src_dir_path = self.dir.path();

        let dest_dir_path = self.dir_path;

        let mut entries = fs::read_dir(src_dir_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let src_path = entry.path();

            let dest_path = dest_dir_path.join(entry.file_name());

            if !src_path.is_dir_exists_nofollow().await? {
                continue;
            }

            if dest_path.is_dir_exists_nofollow().await? {
                fs::remove_dir_all(&dest_path).await?;
            }

            fs::rename(src_path, dest_path).await?;
        }

        self.dir.close()?;

        for symlink_path in self.symlink_paths {
            dest_dir_path
                .create_relative_symlink_atomically_at(symlink_path)
                .await?;
        }

        Ok(())
    }
}
