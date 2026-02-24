use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_recursion::async_recursion;
use etcetera::AppStrategy;
use foyer::{Cache, CacheBuilder};
use futures::stream::{self, StreamExt, TryStreamExt};
use serde_json::Value;
use tokio::fs;

use super::Registrable;
use crate::{
    context::Context,
    package::{
        Packageable,
        formula::{RawFormula, ResolvedFormula},
    },
};

pub struct FormulaRegistry {
    store: Cache<String, Arc<ResolvedFormula>>,

    context: Arc<Context>,
}

impl Registrable for FormulaRegistry {
    type ResolvedPackage = ResolvedFormula;

    const API_URL: &str = "https://formulae.brew.sh/api/formula/{}.json";

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

    async fn resolve(self: Arc<Self>, package: String) -> Result<Arc<Self::ResolvedPackage>> {
        let resolved_formula = self.resolve_with_stack(package, Vec::new()).await?;

        Ok(resolved_formula)
    }
}

impl FormulaRegistry {
    async fn resolve_with_stack(
        self: Arc<Self>,
        package: String,
        mut stack: Vec<String>,
    ) -> Result<Arc<ResolvedFormula>> {
        if stack.contains(&package) {
            let formula = package;

            stack.push(formula);

            let err = anyhow!(
                "Circular formula dependency detected: {}",
                stack
                    .iter()
                    .map(|formula| format!(r#""{formula}""#))
                    .collect::<Vec<_>>()
                    .join(" -> "),
            );

            return Err(err);
        }

        stack.push(package.clone());

        let resolved_formula = self
            .store
            .get_or_fetch(&package, || {
                let this = Arc::clone(&self);

                this.fetch_with_stack(package.clone(), stack)
            })
            .await
            .map(|entry| Arc::clone(entry.value()))?;

        Ok(resolved_formula)
    }

    #[async_recursion]
    async fn fetch_with_stack(
        self: Arc<Self>,
        package: String,
        stack: Vec<String>,
    ) -> Result<Arc<ResolvedFormula>> {
        let url = Self::API_URL.replace("{}", &package);

        let res = self
            .context
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?;

        let value: Value = res.json().await?;

        let bytes = serde_json::to_vec(&value)?;

        let raw_formula: RawFormula = serde_json::from_value(value)?;

        let dependencies = raw_formula.dependencies.iter().cloned();

        let resolved_dependencies = stream::iter(dependencies)
            .map(|dependency| {
                let this = Arc::clone(&self);

                this.resolve_with_stack(dependency, stack.clone())
            })
            .buffer_unordered(*self.context.concurrency_limit)
            .try_collect::<Vec<_>>()
            .await?;

        let resolved_formula = ResolvedFormula::from((raw_formula, resolved_dependencies));
        let resolved_formula = Arc::new(resolved_formula);

        let dir = self
            .context
            .project_dirs()?
            .cache_dir()
            .join("api")
            .join("formula");

        fs::create_dir_all(&dir).await?;

        let file = dir.join(format!("{}.json", resolved_formula.id()));

        fs::write(file, bytes).await?;

        Ok(resolved_formula)
    }
}
