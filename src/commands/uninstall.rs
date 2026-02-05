use async_trait::async_trait;
use clap::Args;
use color_eyre::eyre::Result;
use futures::future;

use crate::{
    commands::{Resolution, Runner},
    context::Context,
    package::Package,
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
    async fn run(&self, context: &Context) -> Result<()> {
        let strategy = self.resolution.strategy();

        let packages = self
            .packages
            .iter()
            .map(|package| Package::resolve(package, context, &strategy));
        let packages = future::try_join_all(packages).await?;

        Ok(())
    }
}
