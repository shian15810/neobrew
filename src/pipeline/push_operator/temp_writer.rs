use std::path::PathBuf;

use anyhow::Result;
use bytes::Bytes;
use tempfile::NamedTempFile;
use tokio::{
    fs::{self, File},
    io::{AsyncWriteExt as _, BufWriter},
};

use super::{super::handler, PushOperator};
use crate::ext::{
    std::path::PathExt as _,
    tokio::{fs::FileExt as _, path::PathExt as _},
};

pub(crate) struct TempWriter {
    file: NamedTempFile,
    buf_file: BufWriter<File>,

    file_path: PathBuf,
    symlink_paths: Vec<PathBuf>,
}

impl TempWriter {
    pub(crate) async fn create(file_path: PathBuf, symlink_paths: Vec<PathBuf>) -> Result<Self> {
        let file_base_path = file_path.base()?;

        fs::create_dir_all(file_base_path).await?;

        let file = NamedTempFile::new_in(file_base_path)?;

        let async_file = File::open_write(file.path()).await?;

        let buf_file = BufWriter::new(async_file);

        let this = Self {
            file,
            buf_file,

            file_path,
            symlink_paths,
        };

        Ok(this)
    }
}

impl PushOperator for TempWriter {
    type Item = Bytes;
    type Output = TempWriterOutput;

    async fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        self.buf_file.write_all(&chunk).await?;

        Ok(())
    }

    async fn flush(mut self) -> Result<Self::Output> {
        self.buf_file.shutdown().await?;

        let output = TempWriterOutput {
            file: self.file,

            file_path: self.file_path,
            symlink_paths: self.symlink_paths,
        };

        Ok(output)
    }
}

pub(crate) struct TempWriterOutput {
    file: NamedTempFile,

    file_path: PathBuf,
    symlink_paths: Vec<PathBuf>,
}

impl handler::AtomicWriter for TempWriterOutput {
    async fn cleanup(self) -> Result<()> {
        self.file.close()?;

        Ok(())
    }

    async fn persist(self) -> Result<()> {
        let dest_file_path = self.file_path;

        self.file.persist(&dest_file_path)?;

        for symlink_path in self.symlink_paths {
            let symlink_base_path = symlink_path.base()?;

            fs::create_dir_all(symlink_base_path).await?;

            dest_file_path
                .create_relative_symlink_atomically_at(symlink_path)
                .await?;
        }

        Ok(())
    }
}
