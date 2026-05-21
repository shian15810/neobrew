use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_recursion::async_recursion;
use foyer::{Cache, CacheBuilder};
use futures::stream::{self, StreamExt as _, TryStreamExt as _};

use super::Registrable;
use crate::{
    context::Context,
    package::{
        raw::{RawFormula, RawPackage},
        resolved::ResolvedFormula,
    },
};

pub(super) struct FormulaRegistry {
    store: Cache<Arc<str>, Arc<ResolvedFormula>>,

    context: Arc<Context>,
}

impl Registrable for FormulaRegistry {
    type ResolvedPackage = ResolvedFormula;

    const API_URL: &str = "https://formulae.brew.sh/api/formula/{}.json";

    const JSON_URL: &str = "https://formulae.brew.sh/api/formula.json";
    const JWS_JSON_URL: &str = "https://formulae.brew.sh/api/formula.jws.json";
    const TAP_MIGRATIONS_URL: &str = "https://formulae.brew.sh/api/formula_tap_migrations.json";
    const TAP_MIGRATIONS_JWS_URL: &str =
        "https://formulae.brew.sh/api/formula_tap_migrations.jws.json";

    fn new(context: Arc<Context>) -> Self {
        Self {
            store: CacheBuilder::new(usize::MAX).build(),

            context,
        }
    }

    async fn resolve(self: Arc<Self>, package: Arc<str>) -> Result<Arc<Self::ResolvedPackage>> {
        let stack = Vec::new();

        let resolved_formula = self.resolve_with_stack(package, stack).await?;

        Ok(resolved_formula)
    }
}

impl FormulaRegistry {
    async fn resolve_with_stack(
        self: Arc<Self>,
        package: Arc<str>,
        mut stack: Vec<Arc<str>>,
    ) -> Result<Arc<ResolvedFormula>> {
        if stack.contains(&package) {
            let formula = package;

            stack.push(formula);

            let stack = stack
                .into_iter()
                .map(|formula| format!(r#""{formula}""#))
                .collect::<Vec<_>>();
            let stack = stack.join(" -> ");

            let err = anyhow!("Circular package dependency detected: {stack}");

            return Err(err);
        }

        {
            let package = Arc::clone(&package);

            stack.push(package);
        }

        let resolved_formula = self
            .store
            .get_or_fetch(&package, || {
                let this = Arc::clone(&self);

                let package = Arc::clone(&package);

                this.with_stack(package, stack)
            })
            .await?;
        let resolved_formula = resolved_formula.value();
        let resolved_formula = Arc::clone(resolved_formula);

        Ok(resolved_formula)
    }

    #[async_recursion]
    async fn with_stack(
        self: Arc<Self>,
        package: Arc<str>,
        stack: Vec<Arc<str>>,
    ) -> Result<Arc<ResolvedFormula>> {
        let api_url = Self::API_URL.replace("{}", &package);

        let bytes = {
            let _permit = self.context.semaphore.acquire().await?;

            let resp = self.context.client.get(api_url).send().await?;
            let resp = resp.error_for_status()?;

            resp.bytes().await?
        };

        let raw_formula: RawFormula = serde_json::from_slice(&bytes)?;

        let raw_formula_dependencies = raw_formula.dependencies().to_vec();

        let raw_package = RawPackage::Formula(raw_formula);

        {
            let this = Arc::as_ref(&self);

            let context = Arc::as_ref(&self.context);

            this.cache_raw_package_json(&raw_package, bytes, context)
                .await?;
        }

        let resolved_formula_dependencies = stream::iter(raw_formula_dependencies)
            .map(async |raw_formula_dependency| -> Result<_> {
                let this = Arc::clone(&self);

                let raw_formula_dependency = Arc::from(raw_formula_dependency);

                let resolved_formula_dependency = this
                    .resolve_with_stack(raw_formula_dependency, stack.clone())
                    .await?;

                Ok(resolved_formula_dependency)
            })
            .buffer_unordered(*self.context.concurrency_limit)
            .try_collect::<Vec<_>>()
            .await?;

        #[expect(clippy::disallowed_macros)]
        let RawPackage::Formula(raw_formula) = raw_package else {
            unreachable!();
        };

        let resolved_formula = ResolvedFormula::from((raw_formula, resolved_formula_dependencies));
        let resolved_formula = Arc::new(resolved_formula);

        Ok(resolved_formula)
    }
}
