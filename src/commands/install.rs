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
    package::{Packageable as _, ResolvedPackage, ResolvedPackageable as _},
    pipeline::{
        Pipeline,
        pull_operators::Pourer,
        push_operators::{Hasher, Writer},
    },
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

        let mut set: JoinSet<Result<()>> = JoinSet::new();

        let concurrency_limit = *context.concurrency_limit;

        for resolved_package in resolved_packages {
            if set.len() >= concurrency_limit
                && let Some(res) = set.join_next().await
            {
                res??;
            }

            let context = Arc::clone(&context);

            set.spawn(async move {
                let id = resolved_package.id();

                let _version = resolved_package.version();

                let cache = resolved_package.cache().context("Unexpected `None`")?;

                let _sha256 = resolved_package.sha256().context("Unexpected `None`")?;

                let stream = match &resolved_package {
                    ResolvedPackage::Formula(resolved_formula) => {
                        let oci = resolved_formula.oci().context("Unexpected `None`")?;

                        context
                            .oci_client
                            .store_auth_if_needed(oci.registry, &RegistryAuth::Anonymous)
                            .await;

                        let stream = context
                            .oci_client
                            .pull_blob_stream(&oci.reference, &oci.descriptor)
                            .await?;

                        stream.err_into::<anyhow::Error>().left_stream()
                    },
                    ResolvedPackage::Cask(resolved_cask) => {
                        let url = resolved_cask.url();

                        let resp = context.client.get(url).send().await?;
                        let resp = resp.error_for_status()?;

                        let stream = resp.bytes_stream();

                        stream.err_into::<anyhow::Error>().right_stream()
                    },
                };

                let hlist_pat![_hash, _path, _file] = Pipeline::new(stream, context)
                    .fanout(Hasher::new())
                    .fanout(Pourer::new(id))
                    .fanout(Writer::new(cache.file_name)?)
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
