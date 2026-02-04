use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
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

        let resolvers = self
            .packages
            .iter()
            .map(|package| Package::resolve(package, &context));

        let packages = future::join_all(resolvers).await;

        Ok(())
    }
}
