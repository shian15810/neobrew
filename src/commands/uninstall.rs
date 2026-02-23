use std::sync::Arc;

use anyhow::{Error, Result};
use clap::Args;
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
pub struct Uninstall {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,

    #[command(flatten)]
    resolution: Resolution,
}

impl Runner for Uninstall {
    async fn run(self, context: Arc<Context>) -> Result<()> {
        if self.packages.is_empty() {
            return Ok(());
        }

        let registries = Registries::new(Arc::clone(&context));

        let strategy = self.resolution.strategy();

        let packages = registries.resolve(self.packages, strategy).await?;

        let mut set = JoinSet::new();

        let concurrency_limit = context.concurrency_limit();

        for package in packages {
            if set.len() >= concurrency_limit
                && let Some(res) = set.join_next().await
            {
                res??;
            }

            let context = Arc::clone(&context);

            set.spawn(async move {
                let id = package.id();

                let stream = context
                    .client()
                    .get("https://httpbin.org/json")
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes_stream();

                let (hash, path, file) = Pipeline::new(stream, context)
                    .fanout(Hasher::new())
                    .fanout(Pourer::new(id))
                    .fanout(Writer::new(format!("{id}.json"))?)
                    .spawn()
                    .await?;

                dbg!(hash, path, file);

                Ok::<_, Error>(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}
