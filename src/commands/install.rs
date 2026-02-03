use anyhow::Result;
use async_trait::async_trait;
use clap::Args;

use crate::{commands::Runner, context::config::Config};

#[derive(Args)]
pub struct Install {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,
}

#[async_trait]
impl Runner for Install {
    async fn run(&self, config: &Config) -> Result<()> {
        println!("Install packages: {:?}", self.packages);

        Ok(())
    }
}
