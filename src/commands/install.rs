use std::sync::Arc;

use anyhow::{Context as _, Result};
use clap::Args;
use frunk::hlist_pat;
use futures::prelude::stream::{StreamExt as _, TryStreamExt as _};
use oci_client::secrets::RegistryAuth;
use tokio::task::JoinSet;

use super::{Resolution, Runner};
use crate::{
    context::Context,
    package::{Packageable as _, PreparedPackage, PreparedPackageable as _},
    pipeline::{Hasher, Pipeline, Pourer, Writer},
    registry::Registry,
};

#[derive(Args)]
pub(super) struct Install {
    #[command(flatten)]
    resolution: Resolution,

    #[arg(value_name = "PACKAGE")]
    packages: Vec<String>,
}

impl Runner for Install {
    async fn run_concurrent(self, context: Arc<Context>) -> Result<()> {
        if self.packages.is_empty() {
            return Ok(());
        }

        let registry = {
            let context = Arc::clone(&context);

            Registry::new(context)
        };

        let strategy = self.resolution.strategy();

        let resolved_packages = registry.resolve(self.packages, strategy).await?;

        #[cfg(debug_assertions)]
        let prepared_packages = resolved_packages
            .into_iter()
            .map(PreparedPackage::try_from)
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let prepared_packages = resolved_packages
            .into_iter()
            .map(PreparedPackage::try_from)
            .collect::<Result<Vec<_>>>()?;

        let mut set: JoinSet<Result<()>> = JoinSet::new();

        for prepared_package in prepared_packages {
            if set.len() >= *context.concurrency_limit
                && let Some(res) = set.join_next().await
            {
                res??;
            }

            let context = Arc::clone(&context);

            set.spawn(async move {
                let id = prepared_package.id();

                let version = prepared_package.version();

                let _fetch_sha256 = prepared_package.fetch_sha256();

                let fetch_cache = {
                    let context = Arc::as_ref(&context);

                    prepared_package
                        .fetch_cache(context)
                        .context("Unexpected `None`")?
                };

                let fetch_dest = {
                    let context = Arc::as_ref(&context);

                    prepared_package.fetch_dest(context)
                };

                let fetch_stream = match &prepared_package {
                    PreparedPackage::Formula(prepared_formula) => {
                        let fetch_oci =
                            prepared_formula.fetch_oci().context("Unexpected `None`")?;

                        context
                            .oci_client
                            .store_auth_if_needed(fetch_oci.registry, &RegistryAuth::Anonymous)
                            .await;

                        let fetch_stream = context
                            .oci_client
                            .pull_blob_stream(&fetch_oci.reference, &fetch_oci.descriptor)
                            .await?;
                        let fetch_stream = fetch_stream.err_into::<anyhow::Error>();

                        fetch_stream.left_stream()
                    },

                    PreparedPackage::Cask(prepared_cask) => {
                        let fetch_url = prepared_cask.fetch_url();

                        let fetch_resp = context.client.get(fetch_url).send().await?;
                        let fetch_resp = fetch_resp.error_for_status()?;

                        let fetch_stream = fetch_resp.bytes_stream();
                        let fetch_stream = fetch_stream.err_into::<anyhow::Error>();

                        fetch_stream.right_stream()
                    },
                };

                let hlist_pat![_hashed_sha256, (), ()] = Pipeline::new(fetch_stream, context)
                    .fanout(Hasher::new())
                    .fanout(Writer::new(fetch_cache)?)
                    .fanout(Pourer::new(id.to_owned(), version.to_owned(), fetch_dest))
                    .run_parallel()
                    .await?;

                Ok(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}
