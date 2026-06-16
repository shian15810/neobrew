use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use async_recursion::async_recursion;
use foyer::{Cache, CacheBuilder};
use futures::future;

use super::{
    FormulaRegistry,
    RegistryExt,
    RegistryJsonExt,
    compatibility::{CaskCompatibility as _, Compatibility},
};
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    package::{
        PackageExt as _,
        raw::cask::RawCask,
        resolved::{ResolvedPackageExt as _, cask::ResolvedCask},
    },
};

pub(super) struct CaskRegistry {
    store: Cache<Arc<str>, Arc<ResolvedCask>>,

    formula_registry: Arc<FormulaRegistry>,

    compatibility: Arc<Compatibility>,

    context: Arc<Context>,
}

impl CaskRegistry {
    pub(super) fn new(
        formula_registry: Arc<FormulaRegistry>,
        compatibility: Arc<Compatibility>,
        context: Arc<Context>,
    ) -> Self {
        Self {
            store: CacheBuilder::new(usize::MAX).build(),

            formula_registry,

            compatibility,

            context,
        }
    }
}

impl RegistryExt for CaskRegistry {
    type ResolvedPackage = ResolvedCask;

    const API_URL: &str = "https://formulae.brew.sh/api/cask/{}.json";

    const JSON_URL: &str = "https://formulae.brew.sh/api/cask.json";
    const JWS_JSON_URL: &str = "https://formulae.brew.sh/api/cask.jws.json";

    const TAP_MIGRATIONS_URL: &str = "https://formulae.brew.sh/api/cask_tap_migrations.json";
    const TAP_MIGRATIONS_JWS_URL: &str =
        "https://formulae.brew.sh/api/cask_tap_migrations.jws.json";

    async fn resolve(self: Arc<Self>, package: Arc<str>) -> anyhow::Result<Arc<ResolvedCask>> {
        let stack = Vec::new();

        let resolved_cask = self.resolve_with_stack(package, stack).await?;

        Ok(resolved_cask)
    }

    async fn resolve_with_stack(
        self: Arc<Self>,
        package: Arc<str>,
        mut stack: Vec<Arc<str>>,
    ) -> anyhow::Result<Arc<ResolvedCask>> {
        if stack.contains(&package) {
            let cask = package;

            stack.push(cask);

            let stack = stack
                .into_iter()
                .map(|cask| format!(r#""{cask}""#))
                .collect::<Vec<_>>();
            let stack = stack.join(" -> ");

            let err = anyhow!("Circular cask dependency detected: {stack}");

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

        let resolved_cask = Arc::clone(entry.value());

        Ok(resolved_cask)
    }

    #[async_recursion]
    async fn fetch_with_stack(
        self: Arc<Self>,
        package: Arc<str>,
        stack: Vec<Arc<str>>,
    ) -> anyhow::Result<Arc<ResolvedCask>> {
        let api_url = Self::API_URL.replace("{}", &package);

        let resp = self.context.client.get(api_url).send().await?;
        let resp = resp.error_for_status()?;

        let bytes = resp.bytes().await?;

        let mut raw_cask: RawCask = serde_json::from_slice(&bytes)?;

        self.save_json(raw_cask.id(), bytes).await?;

        raw_cask = raw_cask.squash_variations(&self.context)?;

        let raw_dependencies = raw_cask
            .dependencies()
            .iter()
            .map(|raw_dependency| {
                let raw_dependency = raw_dependency.as_str();

                Arc::from(raw_dependency)
            })
            .collect::<Vec<_>>();

        let raw_formula_dependencies = raw_cask
            .formula_dependencies()
            .iter()
            .map(|raw_formula_dependency| {
                let raw_formula_dependency = raw_formula_dependency.as_str();

                Arc::from(raw_formula_dependency)
            })
            .collect::<Vec<_>>();

        let resolved_dependencies_futs = raw_dependencies.into_iter().map(async |raw_dependency| {
            let this = Arc::clone(&self);

            let resolved_dependency = this
                .resolve_with_stack(raw_dependency, stack.clone())
                .await?;

            anyhow::Ok(resolved_dependency)
        });

        let resolved_formula_dependencies_futs =
            raw_formula_dependencies
                .into_iter()
                .map(async |raw_formula_dependency| {
                    let formula_registry = Arc::clone(&self.formula_registry);

                    let resolved_formula_dependency = formula_registry
                        .resolve_with_stack(raw_formula_dependency, stack.clone())
                        .await?;

                    anyhow::Ok(resolved_formula_dependency)
                });

        let resolved_dependencies_fut = future::try_join_all(resolved_dependencies_futs);

        let resolved_formula_dependencies_fut =
            future::try_join_all(resolved_formula_dependencies_futs);

        let (dependencies, formula_dependencies) =
            futures::try_join!(resolved_dependencies_fut, resolved_formula_dependencies_fut)?;

        let is_compatible = self.compatibility.is_cask_compatible(&raw_cask);

        let resolved_cask = ResolvedCask::from((raw_cask, dependencies, formula_dependencies));
        let resolved_cask = Arc::new(resolved_cask);

        resolved_cask.set_is_compatible(is_compatible);

        Ok(resolved_cask)
    }
}

impl RegistryJsonExt for CaskRegistry {
    fn json_path(&self, id: &str) -> PathBuf {
        let file_name = format!("{id}.json");

        let cache_dir_path = self.context.homebrew_dirs.cache_dir();

        cache_dir_path.join("api/cask").join(file_name)
    }
}
