use std::path::Path;

use async_trait::async_trait;
use bytes::Bytes;
use tempfile::NamedTempFile;
use tokio::{
    fs::{self, File},
    io::{AsyncWriteExt as _, BufWriter},
};

use super::{
    super::state_store::{Stage, WrittenOutput},
    PushConnector,
};
use crate::{
    ext::{
        std::path::PathExt as _,
        tokio::{fs::FileExt as _, path::PathExt as _},
    },
    package::prepared::{Download, PreparedPackage, PreparedPackageable as _},
};

pub(crate) struct Writer;

impl Writer {
    async fn persist(
        self,
        dest_file: NamedTempFile,
        dest_file_path: &Path,
        dest_link_path: &Path,
    ) -> anyhow::Result<()> {
        dest_file.persist(dest_file_path)?;

        let dest_link_base_path = dest_link_path.base()?;

        fs::create_dir_all(dest_link_base_path).await?;

        dest_file_path
            .create_relative_link_atomically_at(dest_link_path)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl PushConnector for Writer {
    type State = (BufWriter<File>, NamedTempFile);
    type Staging = NamedTempFile;
    type Output = WrittenOutput;

    fn should_run(&self, prepared_package: &PreparedPackage<Download>) -> bool {
        let download = prepared_package.download();

        !download.is_verified()
    }

    async fn on_skip_run(
        mut self,
        prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Option<Self::Output>> {
        let download = prepared_package.download();

        let dest_file_path = download.file_path();

        let dest_link_path = download.link_path();

        let output = WrittenOutput {
            dest_file_path: dest_file_path.to_owned(),
            dest_link_path: dest_link_path.to_owned(),
        };

        Ok(Some(output))
    }

    async fn init(
        &self,
        prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Self::State> {
        let download = prepared_package.download();

        let dest_file_path = download.file_path();

        let dest_file_base_path = dest_file_path.base()?;

        fs::create_dir_all(dest_file_base_path).await?;

        let dest_file = NamedTempFile::new_in(dest_file_base_path)?;

        let dest_file_path = dest_file.path();

        let async_dest_file = File::open_write(dest_file_path).await?;

        let buf_async_dest_file = BufWriter::new(async_dest_file);

        let state = (buf_async_dest_file, dest_file);

        Ok(state)
    }

    async fn feed(&self, state: &mut Self::State, chunk: Bytes) -> anyhow::Result<()> {
        let (buf_async_dest_file, _dest_file) = state;

        buf_async_dest_file.write_all(&chunk).await?;

        Ok(())
    }

    async fn flush(&self, state: Self::State) -> anyhow::Result<Self::Staging> {
        let (mut buf_async_dest_file, dest_file) = state;

        buf_async_dest_file.shutdown().await?;

        let async_dest_file = buf_async_dest_file.get_mut();

        async_dest_file.shutdown().await?;

        let staging = dest_file;

        Ok(staging)
    }

    async fn on_final_run(
        self,
        staging: Self::Staging,
        prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Self::Output> {
        let dest_file = staging;

        let download = prepared_package.download();

        let dest_file_path = download.file_path();

        let dest_link_path = download.link_path();

        self.persist(dest_file, dest_file_path, dest_link_path)
            .await?;

        let output = WrittenOutput {
            dest_file_path: dest_file_path.to_owned(),
            dest_link_path: dest_link_path.to_owned(),
        };

        Ok(output)
    }

    fn passed_stage(&self, _should_run: bool) -> Option<Stage> {
        Some(Stage::Written)
    }
}
