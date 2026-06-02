mod cask;
mod formula;

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use anyhow::anyhow;
use bytes::Bytes;
use futures::{
    future::{self, FutureExt as _},
    stream::{self, StreamExt as _, TryStreamExt as _},
};
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
    ) -> anyhow::Result<(Vec<ResolvedPackage>, HashSet<String>)> {
        let mut requested_package_ids = HashSet::new();

        let mut resolved_package_ids = HashSet::new();

        let resolved_packages = self.resolve_many(packages).await?;
        let mut resolved_packages = resolved_packages
            .into_iter()
            .flat_map(|resolved_package| {
                let mut resolved_package_iter = resolved_package.iter().peekable();

                if let Some(resolved_package) = resolved_package_iter.peek() {
                    let id = resolved_package.id();
                    let id = id.to_owned();

                    requested_package_ids.insert(id);
                }

                resolved_package_iter
            })
            .filter(|resolved_package| {
                let id = resolved_package.id();
                let id = id.to_owned();

                resolved_package_ids.insert(id)
            })
            .collect::<Vec<_>>();

        for resolved_package in &mut resolved_packages {
            #[expect(clippy::collapsible_if)]
            if let ResolvedPackage::Formula(resolved_formula) = resolved_package {
                if let Some(resolved_formula) = Arc::get_mut(resolved_formula) {
                    resolved_formula.dependencies_mut().clear();
                }
            }
        }

        resolved_packages.sort_by(|left, right| left.id().cmp(right.id()));

        Ok((resolved_packages, requested_package_ids))
    }

    async fn resolve_many(
        self,
        packages: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Vec<ResolvedPackage>> {
        let resolved_packages = stream::iter(packages)
            .map(async |package| {
                let package = Arc::from(package);

                let resolved_package = self.resolve_one(package).await?;

                anyhow::Ok(resolved_package)
            })
            .buffer_unordered(self.context.concurrency_limit)
            .try_collect::<Vec<_>>();
        let resolved_packages = resolved_packages.await?;

        Ok(resolved_packages)
    }

    async fn resolve_one(&self, package: Arc<str>) -> anyhow::Result<ResolvedPackage> {
        let resolved_formula_fut = async {
            let formula_registry = Arc::clone(&self.formula_registry);

            let resolved_formula = formula_registry.resolve(Arc::clone(&package)).await?;
            let resolved_formula = ResolvedPackage::Formula(resolved_formula);

            anyhow::Ok(resolved_formula)
        };
        let resolved_formula_fut = resolved_formula_fut.boxed();

        let resolved_cask_fut = async {
            let cask_registry = Arc::clone(&self.cask_registry);

            let resolved_cask = cask_registry.resolve(Arc::clone(&package)).await?;
            let resolved_cask = ResolvedPackage::Cask(resolved_cask);

            anyhow::Ok(resolved_cask)
        };
        let resolved_cask_fut = resolved_cask_fut.boxed();

        #[expect(clippy::manual_let_else, clippy::single_match_else)]
        let resolved_package =
            match future::select_ok([resolved_formula_fut, resolved_cask_fut]).await {
                Ok((resolved_package, _)) => resolved_package,
                Err(_) => {
                    let err = anyhow!(r#"Package "{package}" not found"#);

                    return Err(err);
                },
            };

        Ok(resolved_package)
    }
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

    async fn resolve(
        self: Arc<Self>,
        package: Arc<str>,
    ) -> anyhow::Result<Arc<Self::ResolvedPackage>>;
}

trait RegistrableJson {
    fn json_path(&self, id: &str) -> PathBuf;

    async fn save_json(&self, id: &str, bytes: Bytes) -> anyhow::Result<()> {
        let dest_file_path = self.json_path(id);

        let dest_base_path = dest_file_path.base()?;

        fs::create_dir_all(dest_base_path).await?;

        let temp_file = NamedTempFile::new_in(dest_base_path)?;

        let mut async_temp_file = File::open_write(temp_file.path()).await?;

        async_temp_file.write_all(&bytes).await?;

        async_temp_file.shutdown().await?;

        temp_file.persist(dest_file_path)?;

        Ok(())
    }
}
