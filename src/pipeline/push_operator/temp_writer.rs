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

pub(crate) struct TempWriterInput {
    pub(crate) file_path: PathBuf,
    pub(crate) symlink_path: Option<PathBuf>,
}

impl TempWriterInput {
    pub(crate) fn new(file_path: PathBuf, symlink_path: Option<PathBuf>) -> Self {
        Self {
            file_path,
            symlink_path,
        }
    }
}

pub(crate) struct TempWriter {
    input: TempWriterInput,
    file: NamedTempFile,
    buf_file: BufWriter<File>,
}

impl TempWriter {
    pub(crate) async fn create(input: TempWriterInput) -> Result<Self> {
        let file_base_path = input.file_path.base()?;

        fs::create_dir_all(file_base_path).await?;

        let file = NamedTempFile::new_in(file_base_path)?;

        let async_file = File::open_write(file.path()).await?;

        let buf_file = BufWriter::new(async_file);

        let this = Self {
            input,
            file,
            buf_file,
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
            input: self.input,
            file: self.file,
        };

        Ok(output)
    }
}

pub(crate) struct TempWriterOutput {
    input: TempWriterInput,
    file: NamedTempFile,
}

impl handler::AtomicWriter for TempWriterOutput {
    async fn cleanup(self) -> Result<()> {
        self.file.close()?;

        Ok(())
    }

    async fn persist(self) -> Result<()> {
        self.file.persist(&self.input.file_path)?;

        if let Some(symlink_path) = self.input.symlink_path {
            self.input
                .file_path
                .create_relative_symlink_atomically_at(symlink_path)
                .await?;
        }

        Ok(())
    }
}
