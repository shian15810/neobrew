use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use async_recursion::async_recursion;
use foyer::{Cache, CacheBuilder};
use futures::future;

use super::{
    RegistryExt,
    RegistryJsonExt,
    compatibility::{Compatibility, FormulaCompatibility as _},
};
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    package::{
        PackageExt as _,
        raw::formula::RawFormula,
        resolved::{ResolvedPackageExt as _, formula::ResolvedFormula},
    },
};

pub(super) struct FormulaRegistry {
    store: Cache<Arc<str>, Arc<ResolvedFormula>>,

    compatibility: Arc<Compatibility>,

    context: Arc<Context>,
}

impl FormulaRegistry {
    pub(super) fn new(compatibility: Arc<Compatibility>, context: Arc<Context>) -> Self {
        Self {
            store: CacheBuilder::new(usize::MAX).build(),

            compatibility,

            context,
        }
    }
}

impl RegistryExt for FormulaRegistry {
    type ResolvedPackage = ResolvedFormula;

    const API_URL: &str = "https://formulae.brew.sh/api/formula/{}.json";

    const JSON_URL: &str = "https://formulae.brew.sh/api/formula.json";
    const JWS_JSON_URL: &str = "https://formulae.brew.sh/api/formula.jws.json";

    const TAP_MIGRATIONS_URL: &str = "https://formulae.brew.sh/api/formula_tap_migrations.json";
    const TAP_MIGRATIONS_JWS_URL: &str =
        "https://formulae.brew.sh/api/formula_tap_migrations.jws.json";

    async fn resolve(self: Arc<Self>, package: Arc<str>) -> anyhow::Result<Arc<ResolvedFormula>> {
        let stack = Vec::new();

        let resolved_formula = self.resolve_with_stack(package, stack).await?;

        Ok(resolved_formula)
    }

    async fn resolve_with_stack(
        self: Arc<Self>,
        package: Arc<str>,
        mut stack: Vec<Arc<str>>,
    ) -> anyhow::Result<Arc<ResolvedFormula>> {
        if stack.contains(&package) {
            let formula = package;

            stack.push(formula);

            let stack = stack
                .into_iter()
                .map(|formula| format!(r#""{formula}""#))
                .collect::<Vec<_>>();
            let stack = stack.join(" -> ");

            let err = anyhow!("Circular formula dependency detected: {stack}");

            return Err(err);
        }

        stack.push(Arc::clone(&package));

        let entry = self
            .store
            .get_or_fetch(&package, || {
                let this = Arc::clone(&self);

                this.fetch_with_stack(Arc::clone(&package), stack)
            })
            .await?;

        let resolved_formula = Arc::clone(entry.value());

        Ok(resolved_formula)
    }

    #[async_recursion]
    async fn fetch_with_stack(
        self: Arc<Self>,
        package: Arc<str>,
        stack: Vec<Arc<str>>,
    ) -> anyhow::Result<Arc<ResolvedFormula>> {
        let api_url = Self::API_URL.replace("{}", &package);

        let resp = self.context.client.get(api_url).send().await?;
        let resp = resp.error_for_status()?;

        let bytes = resp.bytes().await?;

        let raw_formula: RawFormula = serde_json::from_slice(&bytes)?;

        self.save_json(raw_formula.id(), bytes).await?;

        let raw_dependencies = raw_formula
            .dependencies()
            .iter()
            .map(|raw_dependency| Arc::from(raw_dependency.as_str()))
            .collect::<Vec<_>>();

        let resolved_dependencies_futs = raw_dependencies.into_iter().map(async |raw_dependency| {
            let this = Arc::clone(&self);

            let resolved_dependency = this
                .resolve_with_stack(raw_dependency, stack.clone())
                .await?;

            anyhow::Ok(resolved_dependency)
        });

        let dependencies = future::try_join_all(resolved_dependencies_futs).await?;

        let is_compatible = self.compatibility.is_formula_compatible(&raw_formula)?;

        let resolved_formula = ResolvedFormula::from((raw_formula, dependencies));
        let resolved_formula = Arc::new(resolved_formula);

        resolved_formula.set_is_compatible(is_compatible);

        Ok(resolved_formula)
    }
}

impl RegistryJsonExt for FormulaRegistry {
    fn json_path(&self, id: &str) -> PathBuf {
        let file_name = format!("{id}.json");

        let cache_dir_path = self.context.homebrew_dirs.cache_dir();

        cache_dir_path.join("api/formula").join(file_name)
    }
}
