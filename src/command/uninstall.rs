use std::sync::Arc;

use anyhow::Result;
use clap::Args;
use tokio::task::JoinSet;

use super::{Resolution, Runner};
use crate::{context::Context, package::resolved::ResolvedPackage, registries::Registries};

#[derive(Args)]
pub(super) struct Uninstall {
    #[command(flatten)]
    resolution: Resolution,

    #[arg(value_name = "PACKAGE")]
    packages: Vec<String>,
}

impl Runner for Uninstall {
    async fn run_concurrent(self, context: Arc<Context>) -> Result<()> {
        if self.packages.is_empty() {
            return Ok(());
        }

        let resolved_packages = self.resolve_packages(Arc::clone(&context)).await?;

        let mut set = JoinSet::new();

        for _resolved_package in resolved_packages {
            while set.len() >= *context.concurrency_limit {
                if let Some(res) = set.join_next().await {
                    res??;
                }
            }

            let _context = Arc::clone(&context);

            set.spawn(async move { anyhow::Ok(()) });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}

impl Uninstall {
    async fn resolve_packages(self, context: Arc<Context>) -> Result<Vec<ResolvedPackage>> {
        let registries = Registries::new(context);

        let strategy = self.resolution.strategy();

        let resolved_packages = registries.resolve(self.packages, strategy).await?;

        Ok(resolved_packages)
    }
}
