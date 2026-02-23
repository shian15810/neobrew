use std::sync::Arc;

use anyhow::{Result, anyhow};
use enum_dispatch::enum_dispatch;
use futures::stream::{self, StreamExt, TryStreamExt};
use itertools::Itertools;
use once_cell::sync::OnceCell as OnceLock;

use self::{cask::CaskRegistry, formula::FormulaRegistry};
use crate::{
    context::Context,
    package::{Packageable, ResolvedPackage},
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
    ) -> Result<Vec<ResolvedPackage>> {
        let resolved_packages = self.resolve_many(packages, strategy).await?;
        let resolved_packages = resolved_packages
            .into_iter()
            .flat_map(|resolved_package| resolved_package.iter())
            .sorted_by(|a, b| a.id().cmp(b.id()))
            .dedup_by(|a, b| a.id() == b.id())
            .collect::<Vec<_>>();

        Ok(resolved_packages)
    }

    async fn resolve_many(
        self,
        packages: impl IntoIterator<Item = String>,
        strategy: ResolutionStrategy,
    ) -> Result<Vec<ResolvedPackage>> {
        stream::iter(packages)
            .map(|package| self.resolve_one(package, strategy))
            .buffer_unordered(self.context.concurrency_limit())
            .try_collect::<Vec<_>>()
            .await
    }

    async fn resolve_one(
        &self,
        package: String,
        strategy: ResolutionStrategy,
    ) -> Result<ResolvedPackage> {
        if matches!(
            strategy,
            ResolutionStrategy::FormulaOnly | ResolutionStrategy::Both
        ) {
            let formula_registry = self.get_or_init(&self.formula);
            let formula_registry = Arc::clone(formula_registry);

            let resolved_formula = formula_registry.resolve(package.clone()).await;

            if let Ok(resolved_formula) = resolved_formula {
                return Ok(ResolvedPackage::Formula(resolved_formula));
            }
        }

        if matches!(
            strategy,
            ResolutionStrategy::CaskOnly | ResolutionStrategy::Both
        ) {
            let cask_registry = self.get_or_init(&self.cask);
            let cask_registry = Arc::clone(cask_registry);

            let resolved_cask = cask_registry.resolve(package.clone()).await;

            if let Ok(resolved_cask) = resolved_cask {
                return Ok(ResolvedPackage::Cask(resolved_cask));
            }
        }

        Err(anyhow!(r#"Formula or cask "{package}" not found."#))
    }

    fn get_or_init<'a, Reg: Registry>(&self, cell: &'a OnceLock<Arc<Reg>>) -> &'a Arc<Reg> {
        cell.get_or_init(|| {
            let registry = Reg::new(Arc::clone(&self.context));

            Arc::new(registry)
        })
    }
}
