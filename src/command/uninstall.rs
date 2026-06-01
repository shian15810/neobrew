use std::sync::Arc;

use clap::Args;
use tokio::task::JoinSet;

use super::Runner;
use crate::{context::Context, package::resolved::ResolvedPackage, registries::Registries};

#[derive(Args)]
pub(super) struct Uninstall {
    #[arg(value_name = "PACKAGE")]
    packages: Vec<String>,
}

impl Runner for Uninstall {
    async fn run_concurrent(self, context: Arc<Context>) -> anyhow::Result<()> {
        if self.packages.is_empty() {
            return Ok(());
        }

        let resolved_packages = self.resolve_packages(Arc::clone(&context)).await?;

        let mut set = JoinSet::new();

        for _resolved_package in resolved_packages {
            while set.len() >= context.concurrency_limit {
                if let Some(res) = set.join_next().await {
                    res??;
                }
            }

            let _context = Arc::clone(&context);

            set.spawn(async { anyhow::Ok(()) });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}

impl Uninstall {
    async fn resolve_packages(self, context: Arc<Context>) -> anyhow::Result<Vec<ResolvedPackage>> {
        let registries = Registries::new(context);

        let (resolved_packages, _requested_package_ids) = registries.resolve(self.packages).await?;

        Ok(resolved_packages)
    }
}
