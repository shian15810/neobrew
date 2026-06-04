use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use bytes::Bytes;
use futures::future;
use tempfile::NamedTempFile;
use tokio::{
    fs::{self, File},
    io::{AsyncWriteExt as _, BufWriter},
};

use super::{super::channels::PipelineChannels as Channels, PushOperator};
use crate::{
    context::Context,
    ext::{
        std::path::PathExt as _,
        tokio::{fs::FileExt as _, path::PathExt as _},
    },
};

pub(crate) struct FileWriter {
    temp_file: NamedTempFile,
    buf_async_temp_file: BufWriter<File>,

    dest_file_path: PathBuf,
    symlink_paths: Vec<PathBuf>,

    should_run: bool,
}

impl FileWriter {
    pub(crate) async fn try_init(
        dest_file_path: PathBuf,
        symlink_paths: Vec<PathBuf>,
        should_run: bool,
    ) -> anyhow::Result<Self> {
        let dest_base_path = dest_file_path.base()?;

        if should_run {
            fs::create_dir_all(dest_base_path).await?;
        }

        let temp_file = NamedTempFile::new_in(dest_base_path)?;

        let async_temp_file = File::open_write(temp_file.path()).await?;

        let buf_async_temp_file = BufWriter::new(async_temp_file);

        let this = Self {
            temp_file,
            buf_async_temp_file,

            dest_file_path,
            symlink_paths,

            should_run,
        };

        Ok(this)
    }
}

impl PushOperator for FileWriter {
    type Item = Bytes;
    type Output = FileWriterOutput;

    async fn feed(&mut self, chunk: Self::Item) -> anyhow::Result<()> {
        if !self.should_run {
            return Ok(());
        }

        self.buf_async_temp_file.write_all(&chunk).await?;

        Ok(())
    }

    async fn flush(
        mut self,
        channels: Arc<Channels>,
        _context: Arc<Context>,
    ) -> anyhow::Result<Self::Output> {
        let dest_file_path = self.dest_file_path.clone();

        let symlink_paths = self.symlink_paths.clone();

        if !self.should_run {
            self.buf_async_temp_file.shutdown().await?;

            let async_temp_file = self.buf_async_temp_file.get_mut();

            async_temp_file.shutdown().await?;

            self.cleanup()?;

            let output = FileWriterOutput {
                dest_file_path,
                symlink_paths,
            };

            return Ok(output);
        }

        self.buf_async_temp_file.shutdown().await?;

        let async_temp_file = self.buf_async_temp_file.get_mut();

        async_temp_file.shutdown().await?;

        let mut is_verified_rx = channels.is_verified_rx.clone();

        let is_verified = *is_verified_rx.wait_for(Option::is_some).await?;

        if matches!(is_verified, Some(false)) {
            self.cleanup()?;

            let err = anyhow!("Writer failed due to SHA-256 mismatch");

            return Err(err);
        }

        self.persist().await?;

        let output = FileWriterOutput {
            dest_file_path,
            symlink_paths,
        };

        Ok(output)
    }

    fn cleanup(self) -> anyhow::Result<()> {
        self.temp_file.close()?;

        Ok(())
    }

    async fn persist(self) -> anyhow::Result<()> {
        self.temp_file.persist(&self.dest_file_path)?;

        let symlink_path_futs = self.symlink_paths.into_iter().map(async |symlink_path| {
            let symlink_base_path = symlink_path.base()?;

            fs::create_dir_all(symlink_base_path).await?;

            self.dest_file_path
                .create_relative_symlink_atomically_at(symlink_path)
                .await?;

            anyhow::Ok(())
        });

        future::try_join_all(symlink_path_futs).await?;

        Ok(())
    }
}

pub(crate) struct FileWriterOutput {
    pub(in super::super) dest_file_path: PathBuf,
    symlink_paths: Vec<PathBuf>,
}
