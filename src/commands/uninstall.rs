use std::sync::Arc;

use anyhow::{Error, Result};
use async_trait::async_trait;
use clap::Args;
use tokio::task::JoinSet;

use super::{Resolution, Runner};
use crate::{
    context::Context,
    pipeline::{
        Pipeline,
        operator::{Hasher, Writer},
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

#[async_trait]
impl Runner for Uninstall {
    async fn run(self, context: Arc<Context>) -> Result<()> {
        let registries = Registries::new(Arc::clone(&context));

        let strategy = self.resolution.strategy();

        let packages = registries.resolve(self.packages, strategy).await?;

        let mut set = JoinSet::new();

        for package in packages {
            let context = Arc::clone(&context);

            set.spawn(async move {
                let id = package.id();

                let stream = context
                    .http_client()
                    .get("https://httpbin.org/json")
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes_stream();

                let (hash, file) = Pipeline::new(context)
                    .fanout(Hasher::new())
                    .fanout(Writer::new(format!("{id}.json"))?)
                    .send_all(stream)
                    .await?;

                dbg!(hash, file);

                Ok::<_, Error>(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}
