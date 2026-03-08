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
    registries::Registries,
};

#[derive(Args)]
pub struct Install {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,

    #[command(flatten)]
    resolution: Resolution,
}

impl Runner for Install {
    async fn run(self, context: Arc<Context>) -> Result<()> {
        if self.packages.is_empty() {
            return Ok(());
        }

        let registries = {
            let context = Arc::clone(&context);

            Registries::new(context)
        };

        let strategy = self.resolution.strategy();

        let resolved_packages = registries.resolve(self.packages, strategy).await?;

        let mut set: JoinSet<Result<()>> = JoinSet::new();

        let concurrency_limit = *context.concurrency_limit;

        for resolved_package in resolved_packages {
            if set.len() >= concurrency_limit
                && let Some(res) = set.join_next().await
            {
                res??;
            }

            let context = Arc::clone(&context);

            let _handle = set.spawn(async move {
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
                    .spawn()
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
