use std::{
    io::Cursor,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use async_compression::tokio::bufread::GzipDecoder;
use async_zip::base::read::stream::ZipFileReader;
use futures::future;
use tempfile::TempDir;
use tokio::{
    fs::{self, File},
    io::{self, AsyncBufRead, AsyncRead, AsyncReadExt as _, BufReader},
};
use tokio_tar::Archive;
use tokio_util::compat::FuturesAsyncReadCompatExt as _;

use super::{super::channels::PipelineChannels as Channels, PullOperator};
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    util::ArchiveFormat,
};

pub(crate) struct TempPourer {
    temp_dir: TempDir,

    dest_dir_path: PathBuf,
    symlink_paths: Vec<PathBuf>,

    archive_format: Option<ArchiveFormat>,
}

impl TempPourer {
    pub(crate) async fn try_init(
        dest_dir_path: PathBuf,
        symlink_paths: Vec<PathBuf>,
        archive_format: Option<ArchiveFormat>,
    ) -> anyhow::Result<Self> {
        let dest_base_path = &dest_dir_path;

        fs::create_dir_all(dest_base_path).await?;

        let temp_dir = TempDir::new_in(dest_base_path)?;

        let this = Self {
            temp_dir,

            dest_dir_path,
            symlink_paths,

            archive_format,
        };

        Ok(this)
    }

    async fn extract(
        &self,
        archive_format: &ArchiveFormat,
        dir: &TempDir,
        buf_reader: impl AsyncBufRead + Unpin + Send,
    ) -> anyhow::Result<()> {
        match archive_format {
            ArchiveFormat::Dmg => {},
            ArchiveFormat::TarGz => {
                let gz_decoder = GzipDecoder::new(buf_reader);

                let mut archive = Archive::new(gz_decoder);

                archive.unpack(dir.path()).await?;
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
                        let err = anyhow!(r#"Unsafe ZIP entry: "{src_file_pstr}""#);

                        return Err(err);
                    }

                    if src_file_pstr.ends_with('/') {
                        zip_reader = entry_reader.skip().await?;
                    } else {
                        let dest_base_path = dir.path();

                        let dest_file_path = dest_base_path.join(src_file_path);

                        let dest_base_path = dest_file_path.base()?;

                        fs::create_dir_all(dest_base_path).await?;

                        let mut dest_file = File::create(dest_file_path).await?;

                        io::copy(&mut entry_reader.reader_mut().compat(), &mut dest_file).await?;

                        zip_reader = entry_reader.done().await?;
                    }
                }
            },
        }

        Ok(())
    }
}

impl PullOperator for TempPourer {
    type Output = TempPourerOutput;

    async fn from_reader(&self, reader: impl AsyncRead + Unpin + Send) -> anyhow::Result<()> {
        let mut buf_reader = BufReader::new(reader);

        if let Some(archive_format) = &self.archive_format {
            match archive_format {
                ArchiveFormat::Dmg => {
                    io::copy(&mut buf_reader, &mut io::sink()).await?;
                },
                _ => {
                    self.extract(archive_format, &self.temp_dir, buf_reader)
                        .await?;
                },
            }
        } else {
            let mut peek_buf = [0_u8; ArchiveFormat::PEEK_SIZE];

            buf_reader.read_exact(&mut peek_buf).await?;

            let archive_format = ArchiveFormat::detect(&peek_buf).await?;

            let chained_buf_reader = Cursor::new(peek_buf).chain(buf_reader);

            self.extract(&archive_format, &self.temp_dir, chained_buf_reader)
                .await?;
        }

        Ok(())
    }

    async fn after_drain(
        self,
        channels: Arc<Channels>,
        _context: Arc<Context>,
    ) -> anyhow::Result<Self::Output> {
        let mut is_verified_rx = channels.is_verified_rx.clone();

        let is_verified = *is_verified_rx.wait_for(Option::is_some).await?;

        if matches!(is_verified, Some(false)) {
            self.cleanup()?;

            let err = anyhow!("Pourer failed due to SHA-256 mismatch");

            return Err(err);
        }

        let dest_dir_path = self.dest_dir_path.clone();

        let symlink_paths = self.symlink_paths.clone();

        self.persist().await?;

        let output = TempPourerOutput {
            dest_dir_path,
            symlink_paths,
        };

        Ok(output)
    }

    fn cleanup(self) -> anyhow::Result<()> {
        self.temp_dir.close()?;

        Ok(())
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

            fs::rename(src_dir_path, dest_dir_path).await?;
        }

        self.temp_dir.close()?;

        let symlink_path_futs = self.symlink_paths.into_iter().map(async |symlink_path| {
            let symlink_base_path = symlink_path.base()?;

            fs::create_dir_all(symlink_base_path).await?;

            dest_base_path
                .create_relative_symlink_atomically_at(symlink_path)
                .await?;

            anyhow::Ok(())
        });

        future::try_join_all(symlink_path_futs).await?;

        Ok(())
    }
}

pub(crate) struct TempPourerOutput {
    dest_dir_path: PathBuf,
    symlink_paths: Vec<PathBuf>,
}
