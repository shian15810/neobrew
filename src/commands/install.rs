use std::sync::Arc;

use anyhow::Result;
use clap::Args;
use frunk::hlist_pat;
use tokio::task::JoinSet;

use super::{Resolution, Runner};
use crate::{
    context::Context,
    package::Packageable,
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

                let resp = context
                    .client
                    .get("https://httpbin.org/json")
                    .send()
                    .await?;
                let resp = resp.error_for_status()?;

                let stream = resp.bytes_stream();

                let hlist_pat![hash, path, file] = Pipeline::new(stream, context)
                    .fanout(Hasher::new())
                    .fanout(Pourer::new(id))
                    .fanout(Writer::new(format!("{id}.json"))?)
                    .run_parallel()
                    .await?;

                dbg!(hash, path, file);

                Ok(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}
