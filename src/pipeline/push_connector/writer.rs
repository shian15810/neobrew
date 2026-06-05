use std::path::PathBuf;

use async_trait::async_trait;
use bytes::Bytes;
use tempfile::NamedTempFile;
use tokio::{
    fs::{self, File},
    io::{AsyncWriteExt as _, BufWriter},
    sync::watch,
};

use super::{
    super::state_store::{Stage, StateStore, WrittenOutput},
    PushConnector,
};
use crate::ext::{
    std::path::PathExt as _,
    tokio::{fs::FileExt as _, path::PathExt as _},
};

pub(crate) struct Writer {
    temp_file: NamedTempFile,
    buf_async_temp_file: BufWriter<File>,

    dest_file_path: PathBuf,
    dest_link_path: PathBuf,

    should_run: bool,
}

impl Writer {
    pub(crate) async fn try_init(
        dest_file_path: PathBuf,
        dest_link_path: PathBuf,
        should_run: bool,
    ) -> anyhow::Result<Self> {
        let dest_base_path = dest_file_path.base()?;

        fs::create_dir_all(dest_base_path).await?;

        let temp_file = NamedTempFile::new_in(dest_base_path)?;

        let async_temp_file = File::open_write(temp_file.path()).await?;

        let buf_async_temp_file = BufWriter::new(async_temp_file);

        let this = Self {
            temp_file,
            buf_async_temp_file,

            dest_file_path,
            dest_link_path,

            should_run,
        };

        Ok(this)
    }
}

#[async_trait]
impl PushConnector for Writer {
    type Item = Bytes;
    type Staging = ();
    type Output = WrittenOutput;

    fn should_run(&self) -> bool {
        self.should_run
    }

    async fn on_skip_run(
        mut self,
        state_store_rx: &mut watch::Receiver<StateStore>,
    ) -> anyhow::Result<Self::Output> {
        self.flush().await?;

        state_store_rx
            .wait_for(|state_store| state_store.stage >= Stage::Hashed)
            .await?;

        let dest_file_path = self.dest_file_path.clone();

        let dest_link_path = self.dest_link_path.clone();

        self.cleanup()?;

        let output = WrittenOutput {
            dest_file_path,
            dest_link_path,
        };

        Ok(output)
    }

    async fn feed(&mut self, chunk: Self::Item) -> anyhow::Result<()> {
        self.buf_async_temp_file.write_all(&chunk).await?;

        Ok(())
    }

    async fn flush(&mut self) -> anyhow::Result<Self::Staging> {
        self.buf_async_temp_file.shutdown().await?;

        let async_temp_file = self.buf_async_temp_file.get_mut();

        async_temp_file.shutdown().await?;

        Ok(())
    }

    async fn on_final_run(
        self,
        _staging: Self::Staging,
        state_store_rx: &mut watch::Receiver<StateStore>,
    ) -> anyhow::Result<Self::Output> {
        state_store_rx
            .wait_for(|state_store| state_store.stage >= Stage::Hashed)
            .await?;

        let dest_file_path = self.dest_file_path.clone();

        let dest_link_path = self.dest_link_path.clone();

        self.persist().await?;

        let output = WrittenOutput {
            dest_file_path,
            dest_link_path,
        };

        Ok(output)
    }

    async fn persist(self) -> anyhow::Result<()> {
        self.temp_file.persist(&self.dest_file_path)?;

        let dest_link_base_path = self.dest_link_path.base()?;

        fs::create_dir_all(dest_link_base_path).await?;

        self.dest_file_path
            .create_relative_link_atomically_at(self.dest_link_path)
            .await?;

        Ok(())
    }

    fn cleanup(self) -> anyhow::Result<()> {
        self.temp_file.close()?;

        Ok(())
    }

    fn passed_stage(&self, _should_run: bool) -> Option<Stage> {
        Some(Stage::Written)
    }
}
