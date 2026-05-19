use std::{iter, path::PathBuf, sync::Arc};

use anyhow::{Context as _, Result, anyhow};
use either::Either::{Left, Right};
use enum_dispatch::enum_dispatch;
use pathdiff::diff_paths;
use tokio::fs;

use self::{cask::PreparedCask, formula::PreparedFormula};
pub(crate) use self::{
    cask::{RawCask, ResolvedCask},
    formula::{RawFormula, ResolvedFormula},
};
use crate::context::Context;

mod cask;
mod formula;

#[enum_dispatch]
enum Package {
    Raw(RawPackage),
    Resolved(ResolvedPackage),
    Prepared(PreparedPackage),
}

#[enum_dispatch(Package, RawPackage, ResolvedPackage, PreparedPackage)]
pub(crate) trait Packageable {
    fn id(&self) -> &str;

    fn version(&self) -> &str;
}

impl<Package: Packageable> Packageable for Arc<Package> {
    fn id(&self) -> &str {
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        this.id()
    }

    fn version(&self) -> &str {
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        this.version()
    }
}

#[enum_dispatch]
pub(crate) enum RawPackage {
    Formula(RawFormula),
    Cask(RawCask),
}

#[enum_dispatch(RawPackage)]
pub(crate) trait RawPackageable: Packageable {
    fn json_cache(&self, context: &Context) -> RawPackageJsonCache;
}

pub(crate) struct RawPackageJsonCache {
    pub(crate) file_location_parent: PathBuf,
    pub(crate) file_location: PathBuf,
}

#[enum_dispatch]
pub(crate) enum ResolvedPackage {
    Formula(Arc<ResolvedFormula>),
    Cask(Arc<ResolvedCask>),
}

impl ResolvedPackage {
    pub(crate) fn iter(&self) -> impl Iterator<Item = Self> + use<> {
        match self {
            Self::Formula(formula) => {
                let formulae = formula.iter().map(Self::Formula);

                Left(formulae)
            },

            Self::Cask(cask) => {
                let cask = Arc::clone(cask);

                let casks = iter::once(cask).map(Self::Cask);

                Right(casks)
            },
        }
    }
}

#[enum_dispatch]
pub(crate) enum PreparedPackage {
    Formula(PreparedFormula),
    Cask(PreparedCask),
}

impl TryFrom<ResolvedPackage> for PreparedPackage {
    type Error = Option<anyhow::Error>;

    fn try_from(resolved_package: ResolvedPackage) -> Result<Self, Self::Error> {
        let this = match resolved_package {
            ResolvedPackage::Formula(resolved_formula) => {
                let Some(resolved_formula) = Arc::into_inner(resolved_formula) else {
                    let err =
                        anyhow!("`Arc<ResolvedFormula>` still has multiple strong references");

                    return Err(Some(err));
                };

                let prepared_formula = PreparedFormula::try_from(resolved_formula)?;

                Self::Formula(prepared_formula)
            },

            ResolvedPackage::Cask(resolved_cask) => {
                let Some(resolved_cask) = Arc::into_inner(resolved_cask) else {
                    let err = anyhow!("`Arc<ResolvedCask>` still has multiple strong references");

                    return Err(Some(err));
                };

                let prepared_cask = PreparedCask::from(resolved_cask);

                Self::Cask(prepared_cask)
            },
        };

        Ok(this)
    }
}

#[expect(private_bounds)]
#[enum_dispatch(PreparedPackage)]
pub(crate) trait PreparedPackageable: PreparedPackageableInner {
    fn fetch_sha256(&self) -> &str;

    async fn fetch_cache(&self, context: &Context) -> Result<PreparedPackageFetchCache>;
}

#[enum_dispatch(PreparedPackage)]
trait PreparedPackageableInner: Packageable {
    async fn fetch_cache_inner(
        &self,
        file_name: &str,
        symlink_name: &str,
        symlink_location_parent: PathBuf,
    ) -> Result<PreparedPackageFetchCache> {
        let file_location_parent = symlink_location_parent.join("downloads");

        let file_location = file_location_parent.join(file_name);

        let file_location_exists = fs::try_exists(&file_location).await?;

        let symlink_location_diff =
            diff_paths(&file_location, &symlink_location_parent).context("Failed to diff paths")?;

        let symlink_location = symlink_location_parent.join(symlink_name);

        let symlink_location_tmp = symlink_location.with_extension("tmp");

        let symlink_location_exists = fs::try_exists(&symlink_location).await?;

        let fetch_cache = PreparedPackageFetchCache {
            file_location_parent,
            file_location,
            file_location_exists,

            symlink_location_parent,
            symlink_location_diff,
            symlink_location_tmp,
            symlink_location,
            symlink_location_exists,
        };

        Ok(fetch_cache)
    }
}

impl PreparedPackage {
    pub(crate) async fn fetch_dest(&self, context: &Context) -> Result<PreparedPackageFetchDest> {
        let id = self.id();

        let version = self.version();

        let dest_dir = match self {
            Self::Formula(_) => context.homebrew_dirs.cellar_dir(),
            Self::Cask(_) => context.homebrew_dirs.caskroom_dir(),
        };

        let dir_location_grandparent = dest_dir;

        let dir_location_parent = dir_location_grandparent.join(id);

        let dir_location = dir_location_parent.join(version);

        let dir_location_exists = fs::try_exists(&dir_location).await?;

        let fetch_dest = PreparedPackageFetchDest {
            dir_location_grandparent,
            dir_location_parent,
            dir_location,
            dir_location_exists,
        };

        Ok(fetch_dest)
    }
}

pub(crate) struct PreparedPackageFetchCache {
    pub(crate) file_location_parent: PathBuf,
    pub(crate) file_location: PathBuf,
    pub(crate) file_location_exists: bool,

    pub(crate) symlink_location_parent: PathBuf,
    pub(crate) symlink_location_diff: PathBuf,
    pub(crate) symlink_location_tmp: PathBuf,
    pub(crate) symlink_location: PathBuf,
    pub(crate) symlink_location_exists: bool,
}

#[expect(clippy::struct_field_names)]
pub(crate) struct PreparedPackageFetchDest {
    pub(crate) dir_location_grandparent: PathBuf,
    pub(crate) dir_location_parent: PathBuf,
    pub(crate) dir_location: PathBuf,
    pub(crate) dir_location_exists: bool,
}
