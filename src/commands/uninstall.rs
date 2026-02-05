use async_trait::async_trait;
use clap::Args;
use color_eyre::eyre::Result;

use crate::{
    commands::{Resolution, Runner},
    context::Context,
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
        Ok(())
    }
}
