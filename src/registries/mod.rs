mod cask;
mod formula;

use std::{path::PathBuf, sync::Arc};

use anyhow::{Result, anyhow};
use bytes::Bytes;
use futures::stream::{self, StreamExt as _, TryStreamExt as _};
use itertools::Itertools as _;
use tempfile::NamedTempFile;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt as _,
};

use self::{cask::CaskRegistry, formula::FormulaRegistry};
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::fs::FileExt as _},
    package::{Packageable as _, resolved::ResolvedPackage},
};

pub(crate) struct Registries {
    formula_registry: Arc<FormulaRegistry>,
    cask_registry: Arc<CaskRegistry>,

    context: Arc<Context>,
}

impl Registries {
    pub(crate) fn new(context: Arc<Context>) -> Self {
        let formula_registry = FormulaRegistry::new(Arc::clone(&context));
        let formula_registry = Arc::new(formula_registry);

        let cask_registry = CaskRegistry::new(Arc::clone(&context));
        let cask_registry = Arc::new(cask_registry);

        Self {
            formula_registry,
            cask_registry,

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
            .map(async |package| {
                let package = Arc::from(package);

                let resolved_package = self.resolve_one(package, strategy).await?;

                anyhow::Ok(resolved_package)
            })
            .buffer_unordered(*self.context.concurrency_limit)
            .try_collect::<Vec<_>>();
        let resolved_packages = resolved_packages.await?;

        Ok(resolved_packages)
    }

    async fn resolve_one(
        &self,
        package: Arc<str>,
        strategy: ResolutionStrategy,
    ) -> Result<ResolvedPackage> {
        if matches!(
            strategy,
            ResolutionStrategy::FormulaOnly | ResolutionStrategy::Both,
        ) {
            let formula_registry = Arc::clone(&self.formula_registry);

            if let Ok(resolved_formula) = formula_registry.resolve(Arc::clone(&package)).await {
                let resolved_package = ResolvedPackage::Formula(resolved_formula);

                return Ok(resolved_package);
            }
        }

        if matches!(
            strategy,
            ResolutionStrategy::CaskOnly | ResolutionStrategy::Both,
        ) {
            let cask_registry = Arc::clone(&self.cask_registry);

            if let Ok(resolved_cask) = cask_registry.resolve(Arc::clone(&package)).await {
                let resolved_package = ResolvedPackage::Cask(resolved_cask);

                return Ok(resolved_package);
            }
        }

        let err = anyhow!(r#"Package "{package}" not found"#);

        Err(err)
    }
}

#[derive(Copy, Clone)]
pub(crate) enum ResolutionStrategy {
    FormulaOnly,
    CaskOnly,
    Both,
}

enum Registry {
    Formula(Arc<FormulaRegistry>),
    Cask(Arc<CaskRegistry>),
}

trait Registrable {
    type ResolvedPackage;

    const API_URL: &str;

    const JSON_URL: &str;
    const JWS_JSON_URL: &str;

    const TAP_MIGRATIONS_URL: &str;
    const TAP_MIGRATIONS_JWS_URL: &str;

    fn new(context: Arc<Context>) -> Self;

    async fn resolve(self: Arc<Self>, package: Arc<str>) -> Result<Arc<Self::ResolvedPackage>>;
}

trait RegistrableJson {
    fn json_path(&self, id: &str) -> PathBuf;

    async fn cache_json(&self, id: &str, bytes: Bytes) -> Result<()> {
        let file_path = self.json_path(id);

        let file_base_path = file_path.base()?;

        fs::create_dir_all(file_base_path).await?;

        let file = NamedTempFile::new_in(file_base_path)?;

        let mut async_file = File::open_write(file.path()).await?;

        async_file.write_all(&bytes).await?;

        async_file.shutdown().await?;

        file.persist(file_path)?;

        Ok(())
    }
}
