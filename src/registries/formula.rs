use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_recursion::async_recursion;
use foyer::{Cache, CacheBuilder};
use futures::stream::{self, StreamExt, TryStreamExt};

use super::Registry;
use crate::{
    context::Context,
    package::formula::{Formula, RawFormula},
};

pub struct FormulaRegistry {
    store: Cache<String, Arc<Formula>>,

    context: Arc<Context>,
}

impl FormulaRegistry {
    async fn resolve_with_stack(
        self: Arc<Self>,
        package: String,
        mut stack: Vec<String>,
    ) -> Result<Arc<Formula>> {
        if stack.contains(&package) {
            stack.push(package);

            return Err(anyhow!(
                "Circular dependency detected: {}",
                stack.join(" -> ")
            ));
        }

        stack.push(package.clone());

        let formula = self
            .store
            .get_or_fetch(&package, || {
                let this = Arc::clone(&self);

                this.fetch_with_stack(package.clone(), stack)
            })
            .await
            .map(|entry| Arc::clone(entry.value()))?;

        Ok(formula)
    }

    #[async_recursion]
    async fn fetch_with_stack(
        self: Arc<Self>,
        package: String,
        stack: Vec<String>,
    ) -> Result<Arc<Formula>> {
        let url = format!("https://formulae.brew.sh/api/formula/{package}.json");

        let raw_formula: RawFormula = self
            .context
            .http_client()
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let dependencies = raw_formula.dependencies.iter().cloned();
        let dependencies = stream::iter(dependencies)
            .map(|dependency| {
                let this = Arc::clone(&self);

                this.resolve_with_stack(dependency, stack.clone())
            })
            .buffer_unordered(*self.context.max_concurrency())
            .try_collect::<Vec<_>>()
            .await?;

        let formula = raw_formula.into_formula(dependencies);

        Ok(Arc::new(formula))
    }
}

impl Registry for FormulaRegistry {
    type Package = Formula;

    const JSON_URL: &str = "https://formulae.brew.sh/api/formula.json";
    const JWS_JSON_URL: &str = "https://formulae.brew.sh/api/formula.jws.json";
    const TAP_MIGRATIONS_URL: &str = "https://formulae.brew.sh/api/formula_tap_migrations.json";
    const TAP_MIGRATIONS_JWS_URL: &str = "https://formulae.brew.sh/api/formula_tap_migrations.jws.json";

    fn new(context: Arc<Context>) -> Self {
        Self {
            store: CacheBuilder::new(usize::MAX).build(),

            context,
        }
    }

    async fn resolve(self: Arc<Self>, package: String) -> Result<Arc<Self::Package>> {
        self.resolve_with_stack(package, Vec::new()).await
    }
}
