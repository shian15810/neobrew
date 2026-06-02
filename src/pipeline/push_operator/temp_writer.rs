use std::path::PathBuf;

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
    temp_file: NamedTempFile,
    buf_temp_file: BufWriter<File>,

    dest_file_path: PathBuf,
    symlink_paths: Vec<PathBuf>,
}

impl TempWriter {
    pub(crate) async fn try_init(
        dest_file_path: PathBuf,
        symlink_paths: Vec<PathBuf>,
    ) -> anyhow::Result<Self> {
        let dest_base_path = dest_file_path.base()?;

        fs::create_dir_all(dest_base_path).await?;

        let temp_file = NamedTempFile::new_in(dest_base_path)?;

        let async_temp_file = File::open_write(temp_file.path()).await?;

        let buf_temp_file = BufWriter::new(async_temp_file);

        let this = Self {
            temp_file,
            buf_temp_file,

            dest_file_path,
            symlink_paths,
        };

        Ok(this)
    }
}

impl PushOperator for TempWriter {
    type Item = Bytes;
    type Output = TempWriterOutput;

    async fn feed(&mut self, chunk: Self::Item) -> anyhow::Result<()> {
        self.buf_temp_file.write_all(&chunk).await?;

        Ok(())
    }

    async fn flush(mut self) -> anyhow::Result<Self::Output> {
        self.buf_temp_file.shutdown().await?;

        let output = TempWriterOutput {
            temp_file: self.temp_file,

            dest_file_path: self.dest_file_path,
            symlink_paths: self.symlink_paths,
        };

        Ok(output)
    }
}

pub(crate) struct TempWriterOutput {
    temp_file: NamedTempFile,

    dest_file_path: PathBuf,
    symlink_paths: Vec<PathBuf>,
}

impl handler::AtomicWriter for TempWriterOutput {
    fn cleanup(self) -> anyhow::Result<()> {
        self.temp_file.close()?;

        Ok(())
    }

    async fn persist(self) -> anyhow::Result<()> {
        self.temp_file.persist(&self.dest_file_path)?;

        for symlink_path in self.symlink_paths {
            let symlink_base_path = symlink_path.base()?;

            fs::create_dir_all(symlink_base_path).await?;

            self.dest_file_path
                .create_relative_symlink_atomically_at(symlink_path)
                .await?;
        }

        Ok(())
    }
}
