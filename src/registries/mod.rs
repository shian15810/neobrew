use std::{iter, sync::Arc};

use anyhow::{Result, anyhow};
use enum_dispatch::enum_dispatch;
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

#[enum_dispatch]
enum RegistryKind {
    Formula(Arc<FormulaRegistry>),
    Cask(Arc<CaskRegistry>),
}

#[enum_dispatch(Arc<RegistryKind>)]
trait Registrable {
    type Package;

    async fn resolve(self: Arc<Self>, package: String) -> Result<Arc<Self::Package>>;
}

trait Registry {
    const JSON_URL: &str;
    const JWS_JSON_URL: &str;
    const TAP_MIGRATIONS_URL: &str;
    const TAP_MIGRATIONS_JWS_URL: &str;

    fn new(context: Arc<Context>) -> Self;
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
        if matches!(
            strategy,
            ResolutionStrategy::FormulaOnly | ResolutionStrategy::Both
        ) {
            let formula_registry = self.get_or_init(&self.formula);
            let formula_registry = Arc::clone(formula_registry);

            let formula = formula_registry.resolve(package.clone()).await;

            if let Ok(formula) = formula {
                return Ok(Package::Formula(formula));
            }
        }

        if matches!(
            strategy,
            ResolutionStrategy::CaskOnly | ResolutionStrategy::Both
        ) {
            let cask_registry = self.get_or_init(&self.cask);
            let cask_registry = Arc::clone(cask_registry);

            let cask = cask_registry.resolve(package.clone()).await;

            if let Ok(cask) = cask {
                return Ok(Package::Cask(cask));
            }
        }

        Err(anyhow!(
            r#"No available formula or cask with the name "{package}"."#
        ))
    }

    fn get_or_init<'a, Reg: Registry>(&self, cell: &'a OnceLock<Arc<Reg>>) -> &'a Arc<Reg> {
        cell.get_or_init(|| {
            let registry = Reg::new(Arc::clone(&self.context));

            Arc::new(registry)
        })
    }
}
