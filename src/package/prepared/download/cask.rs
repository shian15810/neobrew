use std::path::{Path, PathBuf};

use anyhow::Context as _;
use base16ct::HexDisplay;
use bytes::Bytes;
use futures::stream::{BoxStream, StreamExt as _, TryStreamExt as _};
use sha2::{Digest as _, Sha256};
use url::Url;

use super::{
    super::{super::PackageExt as _, cask::PreparedCask},
    DownloadInnerExt,
};
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    ext::std::path::PathExt as _,
    util::archive_format::{ArchiveFormat, ArchiveFormatError},
};

impl DownloadInnerExt for PreparedCask {
    fn url(&self) -> &str {
        self.variation_url()
    }

    async fn file_name_file_path_link_path(
        &self,
        context: &Context,
    ) -> anyhow::Result<(String, PathBuf, PathBuf)> {
        let version = self.version();

        let url = self.variation_url();

        let resp = context.client.get(url).send().await?;
        let resp = resp.error_for_status()?;

        let url = resp.url();
        let url = url.as_str();

        let url_hash = Sha256::digest(url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let url = Url::parse(url)?;

        let mut url_name = url.path_segments().context("Invalid URL")?;
        let url_name = url_name.next_back().context("Empty path segments")?;
        let url_name = url_name.to_owned();

        let url_path = Path::new(&url_name);

        let url_compound_extension = url_path.compound_extension();

        let cache_dir_path = context.homebrew_dirs.cache_dir();

        let file_name = format!("{url_hash}--{url_name}");

        let file_path = cache_dir_path.join("downloads").join(file_name);

        let link_name = match url_compound_extension {
            Some(url_compound_extension) => {
                let url_compound_extension = url_compound_extension
                    .to_str()
                    .context("Invalid compound extension")?;

                format!("{url_name}--{version}.{url_compound_extension}")
            },
            None => format!("{url_name}--{version}"),
        };

        let link_path = cache_dir_path.join("Cask").join(link_name);

        Ok((url_name, file_path, link_path))
    }

    fn expected_sha256(&self) -> &str {
        self.variation_sha256()
    }

    fn archive_format(&self, file_name: &str) -> anyhow::Result<Option<ArchiveFormat>> {
        let archive_format = match ArchiveFormat::try_from(file_name) {
            Ok(archive_format) => archive_format,
            Err(ArchiveFormatError::Unsupported) => return Ok(None),
            Err(ArchiveFormatError::Other(err)) => return Err(err),
        };

        Ok(Some(archive_format))
    }

    async fn fetch_stream_content_length(
        &self,
        context: &Context,
    ) -> anyhow::Result<(BoxStream<'static, anyhow::Result<Bytes>>, Option<u64>)> {
        let url = self.variation_url();

        let resp = context.client.get(url).send().await?;
        let resp = resp.error_for_status()?;

        let content_length = resp.content_length();

        let stream = resp.bytes_stream();
        let stream = stream.err_into();
        let stream = stream.boxed();

        Ok((stream, content_length))
    }
}
