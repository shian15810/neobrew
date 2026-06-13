mod cask;
mod formula;

use std::path::{Path, PathBuf};

use base16ct::HexDisplay;
use bytes::Bytes;
use futures::stream::{BoxStream, StreamExt as _, TryStreamExt as _};
use sha2::{Digest as _, Sha256};
use tokio::{fs::File, io};
use tokio_util::io::{InspectWriter, ReaderStream};

use super::{PreparedCask, PreparedFormula, PreparedPackage, PreparedPackageExt};
use crate::{
    context::Context,
    ext::tokio::{fs::FileExt as _, path::PathExt as _},
    util::archive_format::ArchiveFormat,
};

pub(crate) struct Download {
    url: String,

    file_path: PathBuf,
    link_path: PathBuf,

    actual_sha256: Option<String>,
    expected_sha256: String,

    is_verified: bool,

    archive_format: Option<ArchiveFormat>,

    content_length: Option<u64>,
}

impl Download {
    pub(crate) fn file_path(&self) -> &Path {
        &self.file_path
    }

    pub(crate) fn link_path(&self) -> &Path {
        &self.link_path
    }

    pub(crate) fn expected_sha256(&self) -> &str {
        &self.expected_sha256
    }

    pub(crate) fn is_verified(&self) -> bool {
        self.is_verified
    }

    pub(crate) fn archive_format(&self) -> Option<ArchiveFormat> {
        self.archive_format
    }

    pub(crate) fn content_length(&self) -> Option<u64> {
        self.content_length
    }
}

impl DownloadExt for PreparedPackage {
    async fn prepare_download(
        &self,
        context: &Context,
    ) -> anyhow::Result<(Download, BoxStream<'static, anyhow::Result<Bytes>>)> {
        match self {
            Self::Formula(formula) => formula.prepare_download(context).await,
            Self::Cask(cask) => cask.prepare_download(context).await,
        }
    }
}

impl DownloadInnerExt for PreparedPackage {
    fn url(&self) -> &str {
        match self {
            Self::Formula(formula) => formula.url(),
            Self::Cask(cask) => cask.url(),
        }
    }

    async fn file_path_link_path(&self, context: &Context) -> anyhow::Result<(PathBuf, PathBuf)> {
        match self {
            Self::Formula(formula) => formula.file_path_link_path(context).await,
            Self::Cask(cask) => cask.file_path_link_path(context).await,
        }
    }

    fn expected_sha256(&self) -> &str {
        match self {
            Self::Formula(formula) => formula.expected_sha256(),
            Self::Cask(cask) => cask.expected_sha256(),
        }
    }

    fn archive_format(&self, link_path: &Path) -> anyhow::Result<Option<ArchiveFormat>> {
        match self {
            Self::Formula(formula) => formula.archive_format(link_path),
            Self::Cask(cask) => cask.archive_format(link_path),
        }
    }

    async fn fetch_stream_content_length(
        &self,
        context: &Context,
    ) -> anyhow::Result<(BoxStream<'static, anyhow::Result<Bytes>>, Option<u64>)> {
        match self {
            Self::Formula(formula) => formula.fetch_stream_content_length(context).await,
            Self::Cask(cask) => cask.fetch_stream_content_length(context).await,
        }
    }
}

#[expect(private_bounds)]
pub(super) trait DownloadExt: DownloadInnerExt {
    async fn prepare_download(
        &self,
        context: &Context,
    ) -> anyhow::Result<(Download, BoxStream<'static, anyhow::Result<Bytes>>)> {
        let url = self.url().to_owned();

        let (file_path, link_path) = self.file_path_link_path(context).await?;

        let actual_sha256 = self.actual_sha256(&file_path).await?;

        let expected_sha256 = self.expected_sha256().to_owned();

        let is_verified = self
            .is_verified(
                &file_path,
                &link_path,
                actual_sha256.as_deref(),
                &expected_sha256,
            )
            .await?;

        let archive_format = self.archive_format(&link_path)?;

        let (stream, content_length) = if is_verified {
            self.file_stream_content_length(&file_path).await?
        } else {
            self.fetch_stream_content_length(context).await?
        };

        let download = Download {
            url,

            file_path,
            link_path,

            actual_sha256,
            expected_sha256,

            is_verified,

            archive_format,

            content_length,
        };

        Ok((download, stream))
    }
}

impl DownloadExt for PreparedFormula {}

impl DownloadExt for PreparedCask {}

trait DownloadInnerExt: PreparedPackageExt {
    fn url(&self) -> &str;

    async fn file_path_link_path(&self, context: &Context) -> anyhow::Result<(PathBuf, PathBuf)>;

    async fn actual_sha256(&self, file_path: &Path) -> anyhow::Result<Option<String>> {
        let Some(mut file) = File::open_if_exists(file_path).await? else {
            return Ok(None);
        };

        let mut digest = Sha256::new();

        let mut sink = InspectWriter::new(io::sink(), |chunk| digest.update(chunk));

        io::copy(&mut file, &mut sink).await?;

        let sha256 = digest.finalize();
        let sha256 = HexDisplay(&sha256);
        let sha256 = format!("{sha256:x}");

        Ok(Some(sha256))
    }

    fn expected_sha256(&self) -> &str;

    async fn is_verified(
        &self,
        file_path: &Path,
        link_path: &Path,
        actual_sha256: Option<&str>,
        expected_sha256: &str,
    ) -> anyhow::Result<bool> {
        let is_file_exists = file_path.is_file_exists_nofollow().await?;

        let is_link_exists = link_path.is_link_exists_nofollow().await?;

        let is_link_valid =
            link_path.realpath_or_none().await? == file_path.realpath_or_none().await?;

        let Some(actual_sha256) = actual_sha256 else {
            return Ok(false);
        };

        let is_sha256_equal = actual_sha256 == expected_sha256;

        let is_verified = is_file_exists && is_link_exists && is_link_valid && is_sha256_equal;

        Ok(is_verified)
    }

    fn archive_format(&self, link_path: &Path) -> anyhow::Result<Option<ArchiveFormat>>;

    async fn file_stream_content_length(
        &self,
        file_path: &Path,
    ) -> anyhow::Result<(BoxStream<'static, anyhow::Result<Bytes>>, Option<u64>)> {
        let file = File::open(file_path).await?;

        let metadata = file.metadata().await?;

        let content_length = metadata.len();

        let stream = ReaderStream::new(file);
        let stream = stream.err_into();
        let stream = stream.boxed();

        Ok((stream, Some(content_length)))
    }

    async fn fetch_stream_content_length(
        &self,
        context: &Context,
    ) -> anyhow::Result<(BoxStream<'static, anyhow::Result<Bytes>>, Option<u64>)>;
}
