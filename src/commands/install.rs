use std::sync::Arc;

use anyhow::{Result, anyhow};
use clap::Args;
use frunk::hlist_pat;
use futures::stream::{StreamExt as _, TryStreamExt as _};
use indoc::formatdoc;
use oci_client::secrets::RegistryAuth;
use tokio::task::JoinSet;

use super::{Resolution, Runner};
use crate::{
    context::Context,
    ext::ResultExt as _,
    package::{Packageable as _, PreparedPackage, PreparedPackageable as _},
    pipeline::{AtomicFsHandler as _, Hasher, Pipeline, Pourer, Writer},
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
            .filter_map(Result::transpose_err)
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let prepared_packages = resolved_packages
            .into_iter()
            .map(PreparedPackage::try_from)
            .filter_map(Result::transpose_err)
            .collect::<Result<Vec<_>>>()?;

        let mut set: JoinSet<Result<()>> = JoinSet::new();

        for prepared_package in prepared_packages {
            while set.len() >= *context.concurrency_limit {
                if let Some(res) = set.join_next().await {
                    res??;
                }
            }

            let context = Arc::clone(&context);

            set.spawn(async move {
                let id = prepared_package.id();

                let version = prepared_package.version();

                let fetch_dest = {
                    let context = Arc::as_ref(&context);

                    prepared_package.fetch_dest(context)
                };

                let fetch_cache = {
                    let context = Arc::as_ref(&context);

                    prepared_package.fetch_cache(context).await?
                };

                let fetch_sha256 = prepared_package.fetch_sha256();

                let fetch_stream = match &prepared_package {
                    PreparedPackage::Formula(prepared_formula) => {
                        let Some(fetch_oci) = prepared_formula.fetch_oci() else {
                            return Ok(());
                        };

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

                let pourer = Pourer::from(fetch_dest);

                let writer = Writer::create(fetch_cache).await?;

                let hasher = Hasher::new();

                let hlist_pat![poured_temp_dest, written_temp_cache, hashed_sha256] =
                    Pipeline::new(fetch_stream, context)
                        .fanout(pourer)
                        .fanout(writer)
                        .fanout(hasher)
                        .run_parallel()
                        .await?;

                if hashed_sha256 != fetch_sha256 {
                    poured_temp_dest.cleanup().await?;

                    written_temp_cache.cleanup().await?;

                    let err = anyhow!(formatdoc! {r#"
                        SHA-256 mismatch detected for package "{id}" of version "{version}":

                        Actual  : "{hashed_sha256}"
                        Expected: "{fetch_sha256}""#,
                    });

                    return Err(err);
                }

                poured_temp_dest.persist().await?;

                written_temp_cache.persist().await?;

                Ok(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}
