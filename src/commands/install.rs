use async_trait::async_trait;
use clap::Args;
use color_eyre::eyre::Result;
use futures::future;

use crate::{commands::Runner, context::Context, package::Package};

#[derive(Args)]
pub struct Install {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,
}

#[async_trait]
impl Runner for Install {
    async fn run(&self, context: &Context) -> Result<()> {
        println!("Install packages: {:?}", self.packages);

        let package_resolvers = self
            .packages
            .iter()
            .map(|package| Package::resolve(package, context));

        let packages = future::join_all(package_resolvers).await;

        Ok(())
    }
}
