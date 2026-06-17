use std::path::PathBuf;

use anyhow::Context as _;
use base16ct::HexDisplay;
use bytes::Bytes;
use futures::stream::{BoxStream, StreamExt as _, TryStreamExt as _};
use oci_client::{Reference, manifest::OciDescriptor, secrets::RegistryAuth};
use sha2::{Digest as _, Sha256};
use url::Url;

use super::{super::formula::PreparedFormula, DownloadInnerExt};
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    util::archive_format::ArchiveFormat,
};

impl DownloadInnerExt for PreparedFormula {
    fn url(&self) -> &str {
        &self.url
    }

    #[expect(clippy::unused_async_trait_impl)]
    async fn file_name_file_path_link_path(
        &self,
        context: &Context,
    ) -> anyhow::Result<(String, PathBuf, PathBuf)> {
        let id = &self.id;

        let version_revision = &self.version_revision();

        let rebuild = self.rebuild;

        let bottle = &self.bottle;

        let url = &self.url;

        let url_hash = Sha256::digest(url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let url = Url::parse(url)?;

        let mut url_name = url.path_segments().context("Invalid URL")?;
        let url_name = url_name.next_back().context("Empty path segments")?;
        let url_name = url_name.to_owned();

        let cache_dir_path = context.homebrew_dirs.cache_dir();

        let file_name = format!("{url_hash}--{id}--{version_revision}.{bottle}.bottle");
        let file_name = match rebuild {
            0 => format!("{file_name}.tar.gz"),
            rebuild => format!("{file_name}.{rebuild}.tar.gz"),
        };

        let file_path = cache_dir_path.join("downloads").join(file_name);

        let link_name = format!("{id}--{version_revision}");

        let link_path = cache_dir_path.join(link_name);

        Ok((url_name, file_path, link_path))
    }

    fn expected_sha256(&self) -> &str {
        &self.sha256
    }

    fn archive_format(&self, _file_name: &str) -> anyhow::Result<Option<ArchiveFormat>> {
        let archive_format = ArchiveFormat::TarGzip;

        Ok(Some(archive_format))
    }

    async fn fetch_stream_content_length(
        &self,
        context: &Context,
    ) -> anyhow::Result<(BoxStream<'static, anyhow::Result<Bytes>>, Option<u64>)> {
        const OCI_REGISTRY_URL: &str = "ghcr.io";

        let registry = OCI_REGISTRY_URL;

        let url = &self.url;

        let url_prefix = format!("https://{registry}/v2/");

        let url_postfix = url.strip_prefix(&url_prefix).context("Invalid OCI URL")?;

        let (repository, _) = url_postfix
            .split_once("/blobs/")
            .context("Invalid OCI blob URL")?;

        let sha256 = &self.sha256;

        let digest = format!("sha256:{sha256}");

        let reference =
            Reference::with_digest(registry.to_owned(), repository.to_owned(), digest.clone());

        let descriptor = OciDescriptor {
            digest,

            ..OciDescriptor::default()
        };

        context
            .oci_client
            .store_auth_if_needed(registry, &RegistryAuth::Anonymous)
            .await;

        let sized_stream = context
            .oci_client
            .pull_blob_stream(&reference, &descriptor)
            .await?;

        let content_length = sized_stream.content_length;

        let stream = sized_stream.stream;
        let stream = stream.err_into();
        let stream = stream.boxed();

        Ok((stream, content_length))
    }
}
