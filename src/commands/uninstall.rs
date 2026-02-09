use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use futures::{StreamExt, TryStreamExt, stream};

use super::{Resolution, Runner};
use crate::{context::Context, package::Package};

#[derive(Args)]
pub struct Uninstall {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,

    #[command(flatten)]
    resolution: Resolution,
}

#[async_trait]
impl Runner for Uninstall {
    async fn run(&self, context: Arc<Context>) -> Result<()> {
        let strategy = self.resolution.strategy();

        let packages = self
            .packages
            .iter()
            .cloned()
            .map(|package| Package::resolve(package, Arc::clone(&context), strategy));
        let packages = stream::iter(packages)
            .buffer_unordered(*context.max_concurrency())
            .try_collect::<Vec<_>>()
            .await?;

        Ok(())
    }
}
