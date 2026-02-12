use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::{StreamExt, TryStreamExt, stream};
use once_cell::sync::OnceCell as OnceLock;

use self::{cask::CaskRegistry, formula::FormulaRegistry};
use crate::{context::Context, package::Package};

mod cask;
mod formula;

#[derive(Clone, Copy)]
pub enum ResolutionStrategy {
    FormulaOnly,
    CaskOnly,
    Both,
}

pub struct Registries {
    context: Arc<Context>,

    formula: OnceLock<Arc<FormulaRegistry>>,
    cask: OnceLock<Arc<CaskRegistry>>,
}

impl Registries {
    pub fn new(context: Arc<Context>) -> Self {
        Self {
            context,

            formula: OnceLock::new(),
            cask: OnceLock::new(),
        }
    }

    pub async fn resolve(
        self,
        packages: impl IntoIterator<Item = String>,
        strategy: ResolutionStrategy,
    ) -> Result<Vec<Package>> {
        stream::iter(packages)
            .map(|package| self.resolve_each(package, strategy))
            .buffer_unordered(*self.context.max_concurrency())
            .try_collect::<Vec<_>>()
            .await
    }

    async fn resolve_each(&self, package: String, strategy: ResolutionStrategy) -> Result<Package> {
        match strategy {
            ResolutionStrategy::FormulaOnly => {
                let formula_registry = Arc::clone(self.formula());
                let formula = formula_registry.resolve(package).await?;

                Ok(Package::Formula(formula))
            },

            ResolutionStrategy::CaskOnly => {
                let cask_registry = Arc::clone(self.cask());
                let cask = cask_registry.resolve(package).await?;

                Ok(Package::Cask(cask))
            },

            ResolutionStrategy::Both => {
                let formula_registry = Arc::clone(self.formula());
                let formula = formula_registry.resolve(package.clone()).await;

                if let Ok(formula) = formula {
                    return Ok(Package::Formula(formula));
                }

                let cask_registry = Arc::clone(self.cask());
                let cask = cask_registry.resolve(package.clone()).await;

                if let Ok(cask) = cask {
                    return Ok(Package::Cask(cask));
                }

                Err(anyhow!(
                    "No available formula or cask with the name \"{package}\"."
                ))
            },
        }
    }

    fn formula(&self) -> &Arc<FormulaRegistry> {
        self.formula.get_or_init(|| {
            let context = Arc::clone(&self.context);

            Arc::new(FormulaRegistry::new(context))
        })
    }

    fn cask(&self) -> &Arc<CaskRegistry> {
        self.cask.get_or_init(|| {
            let context = Arc::clone(&self.context);

            Arc::new(CaskRegistry::new(context))
        })
    }
}

trait Registry {
    const JSON_URL: &str;
    const JWS_JSON_URL: &str;
    const TAP_MIGRATIONS_URL: &str;
    const TAP_MIGRATIONS_JWS_URL: &str;

    fn new(context: Arc<Context>) -> Self;
}
