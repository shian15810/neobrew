use std::sync::{Arc, LazyLock};

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

#[derive(Copy, Clone)]
pub enum ResolutionStrategy {
    FormulaOnly,
    CaskOnly,
    Both,
}

#[enum_dispatch]
enum Registry {
    Formula(Arc<FormulaRegistry>),
    Cask(Arc<CaskRegistry>),
}

#[enum_dispatch(Arc<Registry>)]
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

pub struct Registries<FormulaFn, CaskFn> {
    formula: LazyLock<Arc<FormulaRegistry>, FormulaFn>,
    cask: LazyLock<Arc<CaskRegistry>, CaskFn>,

    context: Arc<Context>,
}

impl Registries<(), ()> {
    pub fn new(
        context: Arc<Context>,
    ) -> Registries<impl FnOnce() -> Arc<FormulaRegistry>, impl FnOnce() -> Arc<CaskRegistry>> {
        let formula_context = Arc::clone(&context);

        let cask_context = Arc::clone(&context);

        Registries {
            formula: LazyLock::new(|| {
                let formula_registry = FormulaRegistry::new(formula_context);

                Arc::new(formula_registry)
            }),

            cask: LazyLock::new(|| {
                let cask_registry = CaskRegistry::new(cask_context);

                Arc::new(cask_registry)
            }),

            context,
        }
    }
}

impl<FormulaFn: FnOnce() -> Arc<FormulaRegistry>, CaskFn: FnOnce() -> Arc<CaskRegistry>>
    Registries<FormulaFn, CaskFn>
{
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
            .buffer_unordered(*self.context.concurrency_limit)
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
            ResolutionStrategy::FormulaOnly | ResolutionStrategy::Both,
        ) {
            let formula_registry = Arc::clone(&self.formula);

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
            let cask_registry = Arc::clone(&self.cask);

            let resolved_cask = cask_registry.resolve(package.clone()).await;

            if let Ok(resolved_cask) = resolved_cask {
                let resolved_package = ResolvedPackage::Cask(resolved_cask);

                return Ok(resolved_package);
            }
        }

        let err = anyhow!(r#"Formula or cask "{package}" not found."#);

        Err(err)
    }
}
