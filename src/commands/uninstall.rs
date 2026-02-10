use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use clap::Args;

use super::{Resolution, Runner};
use crate::{context::Context, registries::Registries};

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
        let registries = Registries::new(context);

        let strategy = self.resolution.strategy();

        let _packages = registries
            .resolve(self.packages.iter().cloned(), strategy)
            .await?;

        Ok(())
    }
}
