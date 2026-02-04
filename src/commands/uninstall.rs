use anyhow::Result;
use async_trait::async_trait;
use clap::Args;

use crate::{commands::Runner, context::Context};

#[derive(Args)]
pub struct Uninstall {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,
}

#[async_trait]
impl Runner for Uninstall {
    async fn run(&self, context: &Context) -> Result<()> {
        println!("Uninstall packages: {:?}", self.packages);

        Ok(())
    }
}
