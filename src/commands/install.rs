use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::Arc,
};

use anyhow::{Error, Result};
use async_trait::async_trait;
use clap::Args;
use futures::stream::StreamExt;
use tokio::{sync::mpsc, task::JoinSet};

use super::{Resolution, Runner};
use crate::{context::Context, registries::Registries};

#[derive(Args)]
pub struct Install {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,

    #[command(flatten)]
    resolution: Resolution,
}

#[async_trait]
impl Runner for Install {
    async fn run(self, context: Arc<Context>) -> Result<()> {
        let registries = Registries::new(Arc::clone(&context));

        let strategy = self.resolution.strategy();

        let packages = registries
            .resolve(self.packages.iter().cloned(), strategy)
            .await?;

        let mut set = JoinSet::new();

        for package in packages {
            let (tx, mut rx) = mpsc::channel(32);

            let context = Arc::clone(&context);

            set.spawn(async move {
                let mut stream = context
                    .http_client()
                    .get("https://httpbin.org/json")
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes_stream();

                while let Some(item) = stream.next().await {
                    let chunk = item?;

                    tx.send(chunk).await?;
                }

                Ok::<_, Error>(())
            });

            set.spawn_blocking(move || {
                let id = package.id();

                let file = File::create(format!("{id}.json"))?;
                let mut file = BufWriter::new(file);

                while let Some(chunk) = rx.blocking_recv() {
                    file.write_all(&chunk)?;
                }

                file.flush()?;

                Ok(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}
