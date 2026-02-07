use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use futures::future;

use super::{Resolution, Runner};
use crate::{context::Context, package::Package};

#[derive(Args)]
pub struct Install {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,

    #[command(flatten)]
    resolution: Resolution,
}

#[async_trait]
impl Runner for Install {
    async fn run(&self, context: Arc<Context>) -> Result<()> {
        let strategy = self.resolution.strategy();

        let packages = self
            .packages
            .iter()
            .map(|package| Package::resolve(package, Arc::clone(&context), &strategy));
        let packages = future::try_join_all(packages).await?;

        Ok(())
    }
}
