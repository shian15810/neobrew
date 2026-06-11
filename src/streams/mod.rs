mod cask;
mod formula;

use std::{path::Path, sync::Arc};

use anyhow::Context as _;
use bytes::Bytes;
use futures::stream::{self, StreamExt as _, TryStreamExt as _};
use oci_client::secrets::RegistryAuth;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use self::{cask::CaskStream, formula::FormulaStream};
use crate::{
    context::Context,
    package::prepared::{PreparedPackage, PreparedPackageable as _},
};

pub(crate) struct Streams {
    formula_stream: FormulaStream,
    cask_stream: CaskStream,

    context: Arc<Context>,
}

impl Streams {
    pub(crate) fn new(context: Arc<Context>) -> Self {
        Self {
            formula_stream: FormulaStream::new(Arc::clone(&context)),
            cask_stream: CaskStream::new(Arc::clone(&context)),

            context,
        }
    }

    pub(crate) async fn download(
        &self,
        file_path: &Path,
    ) -> anyhow::Result<(
        impl stream::Stream<Item = anyhow::Result<Bytes>> + Send + 'static,
        u64,
    )> {
        let file = File::open(file_path).await?;

        let metadata = file.metadata().await?;

        let content_length = metadata.len();

        let stream = ReaderStream::new(file);
        let stream = stream.err_into();

        Ok((stream, content_length))
    }

    pub(crate) async fn oci_or_url(
        &self,
        prepared_package: &PreparedPackage,
    ) -> anyhow::Result<(
        impl stream::Stream<Item = anyhow::Result<Bytes>> + Send + 'static,
        Option<u64>,
    )> {
        match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                let (reference, descriptor) = self
                    .formula_stream
                    .oci(prepared_formula)
                    .context("OCI reference for formula not found")?;

                self.context
                    .oci_client
                    .store_auth_if_needed(FormulaStream::OCI_REGISTRY_URL, &RegistryAuth::Anonymous)
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
                let url = prepared_cask.download_url();

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
