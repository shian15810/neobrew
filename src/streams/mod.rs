mod formula;

use std::{path::Path, sync::Arc};

use anyhow::Result;
use bytes::Bytes;
use futures::stream::{self, StreamExt as _, TryStreamExt as _};
use oci_client::secrets::RegistryAuth;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use self::formula::FormulaStream;
use crate::{
    context::Context,
    package::prepared::{PreparedPackage, PreparedPackageable as _},
};

pub(crate) struct Streams {
    context: Arc<Context>,
}

impl Streams {
    pub(crate) fn new(context: Arc<Context>) -> Self {
        Self {
            context,
        }
    }

    pub(crate) async fn cache(
        &self,
        cache_file_path: &Path,
    ) -> Result<impl stream::Stream<Item = Result<Bytes>> + Send + 'static> {
        let cache_file = File::open(cache_file_path).await?;

        let cache_stream = ReaderStream::new(cache_file);
        let cache_stream = cache_stream.err_into();

        Ok(cache_stream)
    }

    pub(crate) async fn api(
        &self,
        prepared_package: &PreparedPackage,
    ) -> Result<Option<impl stream::Stream<Item = Result<Bytes>> + Send + 'static>> {
        let api_stream = match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                let Some((reference, descriptor)) = FormulaStream::oci(prepared_formula) else {
                    return Ok(None);
                };

                self.context
                    .oci_client
                    .store_auth_if_needed(FormulaStream::OCI_REGISTRY, &RegistryAuth::Anonymous)
                    .await;

                let stream = self
                    .context
                    .oci_client
                    .pull_blob_stream(&reference, &descriptor)
                    .await?;
                let stream = stream.err_into();

                stream.left_stream()
            },
            PreparedPackage::Cask(prepared_cask) => {
                let url = prepared_cask.cache_url();

                let resp = self.context.client.get(url).send().await?;
                let resp = resp.error_for_status()?;

                let stream = resp.bytes_stream();
                let stream = stream.err_into();

                stream.right_stream()
            },
        };

        Ok(Some(api_stream))
    }
}
