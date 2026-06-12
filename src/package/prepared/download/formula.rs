use std::path::{Path, PathBuf};

use anyhow::Context as _;
use base16ct::HexDisplay;
use bytes::Bytes;
use futures::stream::{BoxStream, StreamExt as _, TryStreamExt as _};
use oci_client::{Reference, manifest::OciDescriptor, secrets::RegistryAuth};
use sha2::{Digest as _, Sha256};

use super::{
    super::{super::Packageable as _, formula::PreparedFormula},
    DownloadableInner,
};
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    util::ArchiveFormat,
};

impl DownloadableInner for PreparedFormula {
    fn url(&self) -> &str {
        self.bottle_url()
    }

    #[expect(clippy::unused_async_trait_impl)]
    async fn file_path_link_path(&self, context: &Context) -> anyhow::Result<(PathBuf, PathBuf)> {
        let id = self.id();

        let version_revision = self.version_revision();

        let bottle_rebuild = self.bottle_rebuild();

        let bottle_tag = self.bottle_tag();

        let url = self.bottle_url();

        let url_hash = Sha256::digest(url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let cache_dir_path = context.homebrew_dirs.cache_dir();

        let file_name = format!("{url_hash}--{id}--{version_revision}.{bottle_tag}.bottle");
        let file_name = match bottle_rebuild {
            0 => format!("{file_name}.tar.gz"),
            bottle_rebuild => format!("{file_name}.{bottle_rebuild}.tar.gz"),
        };

        let file_path = cache_dir_path.join("downloads").join(file_name);

        let link_name = format!("{id}--{version_revision}");

        let link_path = cache_dir_path.join(link_name);

        Ok((file_path, link_path))
    }

    fn expected_sha256(&self) -> &str {
        self.bottle_sha256()
    }

    fn archive_format(&self, _link_path: &Path) -> anyhow::Result<Option<ArchiveFormat>> {
        let archive_format = ArchiveFormat::TarGz;

        Ok(Some(archive_format))
    }

    async fn fetch_stream_content_length(
        &self,
        context: &Context,
    ) -> anyhow::Result<(BoxStream<'static, anyhow::Result<Bytes>>, Option<u64>)> {
        const OCI_REGISTRY_URL: &str = "ghcr.io";

        let registry = OCI_REGISTRY_URL;

        let url = self.bottle_url();

        let repository = format!("https://{registry}/v2/");
        let repository = url
            .strip_prefix(&repository)
            .context("Invalid OCI repository URL")?;

        let (repository, _) = repository
            .split_once("/blobs/")
            .context("Invalid OCI blob URL")?;

        let sha256 = self.bottle_sha256();

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
