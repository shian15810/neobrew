use std::{
    fs,
    io::{BufWriter, Write as _},
    path::PathBuf,
};

use anyhow::Result;
use bytes::Bytes;
use tempfile::NamedTempFile;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

use super::{super::handler, PushOperator};
use crate::ext::std::path::PathExt as _;

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
    buf_file: BufWriter<NamedTempFile>,

    input: TempWriterInput,
}

impl TempWriter {
    pub(crate) async fn create(input: TempWriterInput) -> Result<Self> {
        let handle = task::spawn_blocking(move || {
            let file_base_path = input.file_path.base()?;

            fs::create_dir_all(file_base_path)?;

            let file = NamedTempFile::new_in(file_base_path)?;

            let this = Self {
                buf_file: BufWriter::new(file),

                input,
            };

            anyhow::Ok(this)
        });
        let handle = AbortOnDropHandle::new(handle);

        let this = handle.await??;

        Ok(this)
    }
}

impl PushOperator for TempWriter {
    type Item = Bytes;
    type Output = TempWriterOutput;

    fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        self.buf_file.write_all(&chunk)?;

        Ok(())
    }

    fn flush(mut self) -> Result<Self::Output> {
        self.buf_file.flush()?;

        let temp_writer_output = TempWriterOutput::try_from(self)?;

        Ok(temp_writer_output)
    }
}

pub(crate) struct TempWriterOutput {
    file: NamedTempFile,

    input: TempWriterInput,
}

impl TryFrom<TempWriter> for TempWriterOutput {
    type Error = anyhow::Error;

    fn try_from(temp_writer: TempWriter) -> Result<Self, Self::Error> {
        let file = temp_writer.buf_file.into_inner()?;

        let this = Self {
            file,

            input: temp_writer.input,
        };

        Ok(this)
    }
}

impl handler::AtomicWriter for TempWriterOutput {
    async fn cleanup(self) -> Result<()> {
        let handle = task::spawn_blocking(move || {
            self.file.close()?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }

    async fn persist(self) -> Result<()> {
        let handle = task::spawn_blocking(move || {
            self.file.persist(&self.input.file_path)?;

            if let Some(symlink_path) = self.input.symlink_path {
                self.input
                    .file_path
                    .create_relative_symlink_atomically_at(symlink_path)?;
            }

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }
}
