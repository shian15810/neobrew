use std::{iter, sync::Arc};

use anyhow::{Result, anyhow};
use futures::stream::{self, StreamExt, TryStreamExt};
use itertools::{Either, Itertools};
use once_cell::sync::OnceCell as OnceLock;

use self::{cask::CaskRegistry, formula::FormulaRegistry};
use crate::{
    context::Context,
    package::{Package, Packageable},
};

mod cask;
mod formula;

#[derive(Copy, Clone)]
pub enum ResolutionStrategy {
    FormulaOnly,
    CaskOnly,
    Both,
}

pub struct Registries {
    formula: OnceLock<Arc<FormulaRegistry>>,
    cask: OnceLock<Arc<CaskRegistry>>,

    context: Arc<Context>,
}

impl Registries {
    pub fn new(context: Arc<Context>) -> Self {
        Self {
            formula: OnceLock::new(),
            cask: OnceLock::new(),

            context,
        }
    }

    pub async fn resolve(
        self,
        packages: impl IntoIterator<Item = String>,
        strategy: ResolutionStrategy,
    ) -> Result<Vec<Package>> {
        let packages = self.resolve_many(packages, strategy).await?;
        let packages = packages
            .into_iter()
            .flat_map(|package| match package {
                Package::Formula(formula) => {
                    let formulae = formula.iter().map(Package::Formula);

                    Either::Left(formulae)
                },

                Package::Cask(cask) => {
                    let casks = iter::once(Package::Cask(cask));

                    Either::Right(casks)
                },
            })
            .sorted_by(|a, b| a.id().cmp(b.id()))
            .dedup_by(|a, b| a.id() == b.id())
            .collect::<Vec<_>>();

        Ok(packages)
    }

    async fn resolve_many(
        self,
        packages: impl IntoIterator<Item = String>,
        strategy: ResolutionStrategy,
    ) -> Result<Vec<Package>> {
        stream::iter(packages)
            .map(|package| self.resolve_one(package, strategy))
            .buffer_unordered(self.context.concurrency_limit())
            .try_collect::<Vec<_>>()
            .await
    }

    async fn resolve_one(&self, package: String, strategy: ResolutionStrategy) -> Result<Package> {
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
    type Package;

    const JSON_URL: &str;
    const JWS_JSON_URL: &str;
    const TAP_MIGRATIONS_URL: &str;
    const TAP_MIGRATIONS_JWS_URL: &str;

    fn new(context: Arc<Context>) -> Self;

    async fn resolve(self: Arc<Self>, package: String) -> Result<Arc<Self::Package>>;
}
