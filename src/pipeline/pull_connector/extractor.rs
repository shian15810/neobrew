use std::{
    collections::HashSet,
    io::Cursor,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context as _, anyhow};
use async_compression::tokio::bufread::{
    BzDecoder,
    GzipDecoder,
    LzmaDecoder,
    XzDecoder,
    ZstdDecoder,
};
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
    super::state_store::{ExtractedOutput, Stage},
    PullConnector,
};
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    package::prepared::{PreparedPackage, PreparedPackageExt as _, download::Download},
    util::archive_format::ArchiveFormat,
};

pub(crate) struct Extractor;

impl Extractor {
    async fn extract(
        &self,
        archive_format: ArchiveFormat,
        buf_reader: impl AsyncBufRead + Unpin,
        src_dir_path: &Path,
    ) -> anyhow::Result<()> {
        match archive_format {
            ArchiveFormat::Tar => {
                let mut archive = Archive::new(buf_reader);

                archive.unpack(src_dir_path).await?;
            },
            ArchiveFormat::TarBzip2 => {
                let bz_decoder = BzDecoder::new(buf_reader);

                let mut archive = Archive::new(bz_decoder);

                archive.unpack(src_dir_path).await?;
            },
            ArchiveFormat::TarGzip => {
                let gzip_decoder = GzipDecoder::new(buf_reader);

                let mut archive = Archive::new(gzip_decoder);

                archive.unpack(src_dir_path).await?;
            },
            ArchiveFormat::TarLzma => {
                let lzma_decoder = LzmaDecoder::new(buf_reader);

                let mut archive = Archive::new(lzma_decoder);

                archive.unpack(src_dir_path).await?;
            },
            ArchiveFormat::TarXz => {
                let xz_decoder = XzDecoder::new(buf_reader);

                let mut archive = Archive::new(xz_decoder);

                archive.unpack(src_dir_path).await?;
            },
            ArchiveFormat::TarZstd => {
                let zstd_decoder = ZstdDecoder::new(buf_reader);

                let mut archive = Archive::new(zstd_decoder);

                archive.unpack(src_dir_path).await?;
            },
            ArchiveFormat::Zip => {
                let mut zip_reader = ZipFileReader::with_tokio(buf_reader);

                let mut created_dir_paths = HashSet::new();

                while let Some(mut entry_reader) = zip_reader.next_with_entry().await? {
                    let src_file_pstr = entry_reader.reader().entry().filename();
                    let src_file_pstr = src_file_pstr.as_str()?;

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
                        let dest_file_path = src_dir_path.join(src_file_path);

                        let dest_file_base_path = dest_file_path.base()?;

                        if !created_dir_paths.contains(dest_file_base_path) {
                            fs::create_dir_all(dest_file_base_path).await?;

                            created_dir_paths.insert(dest_file_base_path.to_owned());
                        }

                        let mut dest_file = File::create(dest_file_path).await?;

                        io::copy(&mut entry_reader.reader_mut().compat(), &mut dest_file).await?;

                        zip_reader = entry_reader.done().await?;
                    }
                }
            },
            ArchiveFormat::Dmg | ArchiveFormat::Pkg => {},
        }

        Ok(())
    }

    async fn persist(self, src_dir: TempDir, dest_dir_path: &Path) -> anyhow::Result<()> {
        let src_dir_path = src_dir.path();

        let mut src_dir_entries = fs::read_dir(src_dir_path).await?;

        while let Some(src_dir_entry) = src_dir_entries.next_entry().await? {
            let src_entry_dir_name = src_dir_entry.file_name();

            let src_entry_dir_path = src_dir_entry.path();

            let dest_entry_dir_path = dest_dir_path.join(src_entry_dir_name);

            if !src_entry_dir_path.is_dir_exists_nofollow().await? {
                continue;
            }

            if dest_entry_dir_path.is_dir_exists_nofollow().await? {
                fs::remove_dir_all(&dest_entry_dir_path).await?;
            }

            fs::rename(&src_entry_dir_path, &dest_entry_dir_path)
                .await
                .with_context(|| {
                    let src_entry_dir_path = src_entry_dir_path.display();

                    let dest_entry_dir_path = dest_entry_dir_path.display();

                    format!(r#"Failed to rename "{src_entry_dir_path}" to "{dest_entry_dir_path}""#)
                })?;
        }

        src_dir.close()?;

        Ok(())
    }
}

#[async_trait]
impl PullConnector for Extractor {
    type Staging = (TempDir, PathBuf, ArchiveFormat);
    type Output = ExtractedOutput;

    fn should_run(&self, prepared_package: &PreparedPackage<Download>) -> bool {
        let download = prepared_package.download();

        let Some(archive_format) = download.archive_format() else {
            return true;
        };

        match archive_format {
            ArchiveFormat::Tar
            | ArchiveFormat::TarBzip2
            | ArchiveFormat::TarGzip
            | ArchiveFormat::TarLzma
            | ArchiveFormat::TarXz
            | ArchiveFormat::TarZstd
            | ArchiveFormat::Zip => true,
            ArchiveFormat::Dmg | ArchiveFormat::Pkg => false,
        }
    }

    async fn from_reader(
        &self,
        reader: &mut (impl AsyncRead + Unpin + Send),
        prepared_package: &PreparedPackage<Download>,
        context: &Context,
    ) -> anyhow::Result<Self::Staging> {
        let dest_dir_path = prepared_package.extract_dir_path(context);

        fs::create_dir_all(&dest_dir_path).await?;

        let src_dir = TempDir::new_in(&dest_dir_path)?;

        let src_dir_path = src_dir.path();

        let mut buf_reader = BufReader::new(reader);

        let download = prepared_package.download();

        #[expect(clippy::single_match_else)]
        let archive_format = match download.archive_format() {
            Some(archive_format) => {
                self.extract(archive_format, buf_reader, src_dir_path)
                    .await?;

                archive_format
            },
            None => {
                let (archive_format, peek_buf) = ArchiveFormat::peek(&mut buf_reader).await?;

                let chained_buf_reader = Cursor::new(peek_buf).chain(buf_reader);

                self.extract(archive_format, chained_buf_reader, src_dir_path)
                    .await?;

                archive_format
            },
        };

        let staging = (src_dir, dest_dir_path, archive_format);

        Ok(staging)
    }

    fn wait_stage(&self) -> Option<Stage> {
        Some(Stage::Hashed)
    }

    async fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output> {
        let (src_dir, dest_dir_path, archive_format) = staging;

        self.persist(src_dir, &dest_dir_path).await?;

        let output = ExtractedOutput {
            dest_dir_path,
            archive_format,
        };

        Ok(output)
    }

    fn passed_stage(&self, should_run: bool) -> Option<Stage> {
        should_run.then_some(Stage::Extracted)
    }
}
