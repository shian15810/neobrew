mod cask;
mod compatibility;
mod formula;

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use anyhow::anyhow;
use bytes::Bytes;
use futures::future::{self, FutureExt as _};
use tempfile::NamedTempFile;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt as _,
};

use self::{
    cask::CaskRegistry,
    compatibility::{Compatibility, CompatibilityExt as _},
    formula::FormulaRegistry,
};
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::fs::FileExt as _},
    package::{
        PackageExt as _,
        resolved::{ResolvedPackage, ResolvedPackageExt as _},
    },
};

pub(crate) struct Registries {
    formula_registry: Arc<FormulaRegistry>,
    cask_registry: Arc<CaskRegistry>,
}

impl Registries {
    pub(crate) async fn try_new(context: Arc<Context>) -> anyhow::Result<Self> {
        let compatibility = Compatibility::try_new(&context).await?;
        let compatibility = Arc::new(compatibility);

        let formula_registry =
            FormulaRegistry::new(Arc::clone(&compatibility), Arc::clone(&context));
        let formula_registry = Arc::new(formula_registry);

        let cask_registry =
            CaskRegistry::new(Arc::clone(&formula_registry), compatibility, context);
        let cask_registry = Arc::new(cask_registry);

        let this = Self {
            formula_registry,
            cask_registry,
        };

        Ok(this)
    }

    pub(crate) async fn resolve(self, packages: &[String]) -> anyhow::Result<Vec<ResolvedPackage>> {
        let mut resolved_package_ids = HashSet::new();

        let resolved_packages = self.resolve_many(packages).await?;

        for resolved_package in &resolved_packages {
            resolved_package.set_is_requested(true);
        }

        #[expect(clippy::redundant_closure_for_method_calls)]
        let mut resolved_packages = resolved_packages
            .into_iter()
            .flat_map(|resolved_package| resolved_package.into_iter())
            .filter(|resolved_package| {
                let id = resolved_package.id();
                let id = id.to_owned();

                resolved_package_ids.insert(id)
            })
            .collect::<Vec<_>>();

        Self::clear_dependencies(&mut resolved_packages);

        resolved_packages.sort_by(|left, right| left.id().cmp(right.id()));

        Ok(resolved_packages)
    }

    async fn resolve_many(self, packages: &[String]) -> anyhow::Result<Vec<ResolvedPackage>> {
        let resolved_packages_fut = packages.iter().map(async |package| {
            let package = package.as_ref();
            let package = Arc::from(package);

            let resolved_package = self.resolve_one(package).await?;

            anyhow::Ok(resolved_package)
        });

        let resolved_packages = future::try_join_all(resolved_packages_fut).await?;

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

        let resolved_package_res =
            future::select_ok([resolved_formula_fut, resolved_cask_fut]).await;

        #[expect(clippy::manual_let_else, clippy::single_match_else)]
        let resolved_package = match resolved_package_res {
            Ok((resolved_package, _)) => resolved_package,
            Err(_) => {
                let err = anyhow!(r#"Package "{package}" not found"#);

                return Err(err);
            },
        };

        Ok(resolved_package)
    }

    fn clear_dependencies(resolved_packages: &mut [ResolvedPackage]) {
        let dependency_count = resolved_packages
            .iter_mut()
            .filter_map(|resolved_package| match resolved_package {
                ResolvedPackage::Formula(resolved_formula) => {
                    let resolved_formula = Arc::get_mut(resolved_formula)?;

                    if !resolved_formula.dependencies().is_empty() {
                        resolved_formula.clear_dependencies();

                        return Some(());
                    }

                    None
                },
                ResolvedPackage::Cask(resolved_cask) => {
                    let resolved_cask = Arc::get_mut(resolved_cask)?;

                    if !resolved_cask.dependencies().is_empty() {
                        resolved_cask.clear_dependencies();

                        return Some(());
                    }

                    if !resolved_cask.formula_dependencies().is_empty() {
                        resolved_cask.clear_formula_dependencies();

                        return Some(());
                    }

                    None
                },
            })
            .count();

        if dependency_count > 0 {
            Self::clear_dependencies(resolved_packages);
        }
    }
}

enum Registry {
    Formula(Arc<FormulaRegistry>),
    Cask(Arc<CaskRegistry>),
}

trait RegistryExt: RegistryJsonExt {
    type ResolvedPackage;

    const API_URL: &str;

    const JSON_URL: &str;
    const JWS_JSON_URL: &str;

    const TAP_MIGRATIONS_URL: &str;
    const TAP_MIGRATIONS_JWS_URL: &str;

    async fn resolve(
        self: Arc<Self>,
        package: Arc<str>,
    ) -> anyhow::Result<Arc<Self::ResolvedPackage>>;

    async fn resolve_with_stack(
        self: Arc<Self>,
        package: Arc<str>,
        stack: Vec<Arc<str>>,
    ) -> anyhow::Result<Arc<Self::ResolvedPackage>>;

    async fn fetch_with_stack(
        self: Arc<Self>,
        package: Arc<str>,
        stack: Vec<Arc<str>>,
    ) -> anyhow::Result<Arc<Self::ResolvedPackage>>;
}

trait RegistryJsonExt {
    fn json_path(&self, id: &str) -> PathBuf;

    async fn save_json(&self, id: &str, bytes: Bytes) -> anyhow::Result<()> {
        let dest_file_path = self.json_path(id);

        let dest_file_base_path = dest_file_path.base()?;

        fs::create_dir_all(dest_file_base_path).await?;

        let json_file = NamedTempFile::new_in(dest_file_base_path)?;

        let json_file_path = json_file.path();

        let mut async_json_file = File::open_write(json_file_path).await?;

        async_json_file.write_all(&bytes).await?;

        async_json_file.shutdown().await?;

        json_file.persist(dest_file_path)?;

        Ok(())
    }
}
