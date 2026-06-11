use std::{
    io::Cursor,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context as _, anyhow};
use async_compression::tokio::bufread::GzipDecoder;
use async_trait::async_trait;
use async_zip::base::read::stream::ZipFileReader;
use tempfile::TempDir;
use tokio::{
    fs::{self, File},
    io::{self, AsyncBufRead, AsyncRead, AsyncReadExt as _, BufReader},
};
use tokio_tar::Archive;
use tokio_util::compat::FuturesAsyncReadCompatExt as _;

use super::{
    super::state_store::{PouredOutput, Stage},
    PullConnector,
};
use crate::{
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    util::ArchiveFormat,
};

pub(crate) struct Pourer {
    temp_dir: TempDir,

    dest_dir_path: PathBuf,
    archive_format: Option<ArchiveFormat>,
}

impl Pourer {
    pub(crate) async fn try_init(
        dest_dir_path: PathBuf,
        archive_format: Option<ArchiveFormat>,
    ) -> anyhow::Result<Self> {
        let dest_base_path = &dest_dir_path;

        fs::create_dir_all(dest_base_path).await?;

        let temp_dir = TempDir::new_in(dest_base_path)?;

        let this = Self {
            temp_dir,

            dest_dir_path,
            archive_format,
        };

        Ok(this)
    }

    async fn extract(
        &self,
        archive_format: &ArchiveFormat,
        buf_reader: impl AsyncBufRead + Unpin,
    ) -> anyhow::Result<()> {
        let dest_base_path = self.temp_dir.path();

        match archive_format {
            ArchiveFormat::TarGz => {
                let gz_decoder = GzipDecoder::new(buf_reader);

                let mut archive = Archive::new(gz_decoder);

                archive.unpack(dest_base_path).await?;
            },
            ArchiveFormat::Zip => {
                let mut zip_reader = ZipFileReader::with_tokio(buf_reader);

                while let Some(mut entry_reader) = zip_reader.next_with_entry().await? {
                    let src_file_pstr = entry_reader.reader().entry().filename().as_str()?;

                    let src_file_path = Path::new(src_file_pstr);

                    let is_src_file_path_safe = src_file_path
                        .components()
                        .all(|component| matches!(component, Component::Normal(_)));

                    if !is_src_file_path_safe {
                        let err = anyhow!(r#"Unsafe ZIP entry detected: "{src_file_pstr}""#);

                        return Err(err);
                    }

                    if src_file_pstr.ends_with('/') {
                        zip_reader = entry_reader.skip().await?;
                    } else {
                        let dest_file_path = dest_base_path.join(src_file_path);

                        let dest_file_base_path = dest_file_path.base()?;

                        fs::create_dir_all(dest_file_base_path).await?;

                        let mut dest_file = File::create(dest_file_path).await?;

                        io::copy(&mut entry_reader.reader_mut().compat(), &mut dest_file).await?;

                        zip_reader = entry_reader.done().await?;
                    }
                }
            },
            ArchiveFormat::Dmg => {},
        }

        Ok(())
    }
}

#[async_trait]
impl PullConnector for Pourer {
    type Staging = ArchiveFormat;
    type Output = PouredOutput;

    fn should_run(&self) -> bool {
        let Some(archive_format) = &self.archive_format else {
            return true;
        };

        match archive_format {
            ArchiveFormat::TarGz | ArchiveFormat::Zip => true,
            ArchiveFormat::Dmg => false,
        }
    }

    fn on_skip_run(self) -> anyhow::Result<Option<Self::Output>> {
        self.cleanup()?;

        Ok(None)
    }

    async fn from_reader(
        &self,
        reader: &mut (impl AsyncRead + Unpin + Send),
    ) -> anyhow::Result<Self::Staging> {
        let mut buf_reader = BufReader::new(reader);

        let archive_format = if let Some(archive_format) = &self.archive_format {
            self.extract(archive_format, buf_reader).await?;

            archive_format.to_owned()
        } else {
            let (archive_format, peek_buf) = ArchiveFormat::peek(&mut buf_reader).await?;

            let chained_buf_reader = Cursor::new(peek_buf).chain(buf_reader);

            self.extract(&archive_format, chained_buf_reader).await?;

            archive_format
        };

        Ok(archive_format)
    }

    fn wait_stage(&self) -> Option<Stage> {
        Some(Stage::Hashed)
    }

    async fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output> {
        let archive_format = staging;

        let dest_dir_path = self.dest_dir_path.clone();

        self.persist().await?;

        let output = PouredOutput {
            dest_dir_path,
            archive_format,
        };

        Ok(output)
    }

    async fn persist(self) -> anyhow::Result<()> {
        let src_base_path = self.temp_dir.path();

        let dest_base_path = self.dest_dir_path;

        let mut src_base_entries = fs::read_dir(src_base_path).await?;

        while let Some(src_base_entry) = src_base_entries.next_entry().await? {
            let src_dir_name = src_base_entry.file_name();

            let src_dir_path = src_base_entry.path();

            let dest_dir_path = dest_base_path.join(src_dir_name);

            if !src_dir_path.is_dir_exists_nofollow().await? {
                continue;
            }

            if dest_dir_path.is_dir_exists_nofollow().await? {
                fs::remove_dir_all(&dest_dir_path).await?;
            }

            fs::rename(&src_dir_path, &dest_dir_path)
                .await
                .with_context(|| {
                    let src_dir_path = src_dir_path.display();

                    let dest_dir_path = dest_dir_path.display();

                    format!(r#"Failed to rename "{src_dir_path}" to "{dest_dir_path}""#)
                })?;
        }

        self.temp_dir.close()?;

        Ok(())
    }

    fn cleanup(self) -> anyhow::Result<()> {
        self.temp_dir.close()?;

        Ok(())
    }

    fn passed_stage(&self, should_run: bool) -> Option<Stage> {
        should_run.then_some(Stage::Poured)
    }
}
