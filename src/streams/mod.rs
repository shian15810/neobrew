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
        file_path: &Path,
    ) -> Result<(
        impl stream::Stream<Item = Result<Bytes>> + Send + 'static,
        u64,
    )> {
        let file = File::open(file_path).await?;

        let content_length = file.metadata().await?.len();

        let stream = ReaderStream::new(file);
        let stream = stream.err_into();

        Ok((stream, content_length))
    }

    pub(crate) async fn api(
        &self,
        prepared_package: &PreparedPackage,
    ) -> Result<(
        impl stream::Stream<Item = Result<Bytes>> + Send + 'static,
        Option<u64>,
    )> {
        match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                let (reference, descriptor) = FormulaStream::oci(prepared_formula)
                    .ok_or_else(|| anyhow::anyhow!("No OCI reference for formula"))?;

                self.context
                    .oci_client
                    .store_auth_if_needed(FormulaStream::OCI_REGISTRY, &RegistryAuth::Anonymous)
                    .await;

                let sized_stream = self
                    .context
                    .oci_client
                    .pull_blob_stream(&reference, &descriptor)
                    .await?;

                let content_length = sized_stream.content_length;

                let stream = sized_stream.stream;
                let stream = stream.err_into();
                let stream = stream.left_stream();

                Ok((stream, content_length))
            },
            PreparedPackage::Cask(prepared_cask) => {
                let url = prepared_cask.cache_url();

                let resp = self.context.client.get(url).send().await?;
                let resp = resp.error_for_status()?;

                let content_length = resp.content_length();

                let stream = resp.bytes_stream();
                let stream = stream.err_into();
                let stream = stream.right_stream();

                Ok((stream, content_length))
            },
        }
    }
}
