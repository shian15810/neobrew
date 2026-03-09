use std::sync::{Arc, OnceLock};

use anyhow::{Result, anyhow};
use enum_dispatch::enum_dispatch;
use futures::stream::{self, StreamExt, TryStreamExt};
use itertools::Itertools;

use self::{cask::CaskRegistry, formula::FormulaRegistry};
use crate::{
    context::Context,
    package::{Packageable, ResolvedPackage},
};

mod cask;
mod formula;

pub(crate) struct Registry {
    formula: OnceLock<Arc<FormulaRegistry>>,
    cask: OnceLock<Arc<CaskRegistry>>,

    context: Arc<Context>,
}

impl Registry {
    pub(crate) fn new(context: Arc<Context>) -> Self {
        Self {
            formula: OnceLock::new(),
            cask: OnceLock::new(),

            context,
        }
    }

    pub(crate) async fn resolve(
        self,
        packages: impl IntoIterator<Item = String>,
        strategy: ResolutionStrategy,
    ) -> Result<Vec<ResolvedPackage>> {
        let resolved_packages = self.resolve_many(packages, strategy).await?;
        let resolved_packages = resolved_packages
            .into_iter()
            .flat_map(|resolved_package| resolved_package.iter())
            .sorted_by(|left, right| left.id().cmp(right.id()))
            .dedup_by(|left, right| left.id() == right.id())
            .collect::<Vec<_>>();

        Ok(resolved_packages)
    }

    async fn resolve_many(
        self,
        packages: impl IntoIterator<Item = String>,
        strategy: ResolutionStrategy,
    ) -> Result<Vec<ResolvedPackage>> {
        let resolved_packages = stream::iter(packages)
            .map(|package| self.resolve_one(package, strategy))
            .buffer_unordered(*self.context.concurrency_limit)
            .try_collect::<Vec<_>>();
        let resolved_packages = resolved_packages.await?;

        Ok(resolved_packages)
    }

    async fn resolve_one(
        &self,
        package: String,
        strategy: ResolutionStrategy,
    ) -> Result<ResolvedPackage> {
        if matches!(
            strategy,
            ResolutionStrategy::FormulaOnly | ResolutionStrategy::Both,
        ) {
            let formula_registry = self.formula();
            let formula_registry = Arc::clone(formula_registry);

            let resolved_formula = formula_registry.resolve(package.clone()).await;

            if let Ok(resolved_formula) = resolved_formula {
                let resolved_package = ResolvedPackage::Formula(resolved_formula);

                return Ok(resolved_package);
            }
        }

        if matches!(
            strategy,
            ResolutionStrategy::CaskOnly | ResolutionStrategy::Both,
        ) {
            let cask_registry = self.cask();
            let cask_registry = Arc::clone(cask_registry);

            let resolved_cask = cask_registry.resolve(package.clone()).await;

            if let Ok(resolved_cask) = resolved_cask {
                let resolved_package = ResolvedPackage::Cask(resolved_cask);

                return Ok(resolved_package);
            }
        }

        let err = anyhow!(r#"Formula or cask "{package}" not found."#);

        Err(err)
    }

    fn formula(&self) -> &Arc<FormulaRegistry> {
        self.formula.get_or_init(|| {
            let context = Arc::clone(&self.context);

            let formula_registry = FormulaRegistry::new(context);

            Arc::new(formula_registry)
        })
    }

    fn cask(&self) -> &Arc<CaskRegistry> {
        self.cask.get_or_init(|| {
            let context = Arc::clone(&self.context);

            let cask_registry = CaskRegistry::new(context);

            Arc::new(cask_registry)
        })
    }
}

#[derive(Copy, Clone)]
pub(crate) enum ResolutionStrategy {
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
    type ResolvedPackage;

    const API_URL: &str;

    const JSON_URL: &str;
    const JWS_JSON_URL: &str;
    const TAP_MIGRATIONS_URL: &str;
    const TAP_MIGRATIONS_JWS_URL: &str;

    fn new(context: Arc<Context>) -> Self;

    async fn resolve(self: Arc<Self>, package: String) -> Result<Arc<Self::ResolvedPackage>>;
}
