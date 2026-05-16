use std::sync::Arc;

use anyhow::Result;
use clap::Args;
use tokio::task::JoinSet;

use super::{Resolution, Runner};
use crate::{context::Context, package::Packageable as _, registry::Registry};

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

        let registry = {
            let context = Arc::clone(&context);

            Registry::new(context)
        };

        let strategy = self.resolution.strategy();

        let resolved_packages = registry.resolve(self.packages, strategy).await?;

        let mut set: JoinSet<Result<()>> = JoinSet::new();

        for resolved_package in resolved_packages {
            if set.len() >= *context.concurrency_limit
                && let Some(res) = set.join_next().await
            {
                res??;
            }

            let _context = Arc::clone(&context);

            set.spawn(async move {
                let _id = resolved_package.id();

                let _version = resolved_package.version();

                Ok(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}
