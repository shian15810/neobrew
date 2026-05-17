use std::{iter, path::PathBuf, sync::Arc};

use anyhow::Context as _;
use enum_dispatch::enum_dispatch;
use itertools::Either;

use self::{cask::PreparedCask, formula::PreparedFormula};
pub(crate) use self::{
    cask::{RawCask, ResolvedCask},
    formula::{RawFormula, ResolvedFormula},
};
use crate::Context;

mod cask;
mod formula;

#[enum_dispatch]
enum Package {
    Raw(RawPackage),
    Resolved(ResolvedPackage),
    Prepared(PreparedPackage),
}

#[enum_dispatch]
enum RawPackage {
    Formula(RawFormula),
    Cask(RawCask),
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

                Either::Left(formulae)
            },

            Self::Cask(cask) => {
                let cask = Arc::clone(cask);

                let casks = iter::once(cask).map(Self::Cask);

                Either::Right(casks)
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
    type Error = anyhow::Error;

    fn try_from(resolved_package: ResolvedPackage) -> Result<Self, Self::Error> {
        let this = match resolved_package {
            ResolvedPackage::Formula(resolved_formula) => {
                let resolved_formula =
                    Arc::into_inner(resolved_formula).context("Unexpected `None`")?;

                let prepared_formula = PreparedFormula::try_from(resolved_formula)?;

                Self::Formula(prepared_formula)
            },
            ResolvedPackage::Cask(resolved_cask) => {
                let resolved_cask = Arc::into_inner(resolved_cask).context("Unexpected `None`")?;

                let prepared_cask = PreparedCask::from(resolved_cask);

                Self::Cask(prepared_cask)
            },
        };

        Ok(this)
    }
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

#[enum_dispatch(RawPackage)]
pub(crate) trait RawPackageable {
    fn json_cache(&self, context: &Context) -> RawPackageJsonCache;
}

impl<RawPackage: RawPackageable> RawPackageable for Arc<RawPackage> {
    fn json_cache(&self, context: &Context) -> RawPackageJsonCache {
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        this.json_cache(context)
    }
}

pub(crate) struct RawPackageJsonCache {
    pub(crate) file_location_parent: PathBuf,
    pub(crate) file_location: PathBuf,
}

#[enum_dispatch(PreparedPackage)]
pub(crate) trait PreparedPackageable {
    fn fetch_sha256(&self) -> &str;

    fn fetch_cache(&self, context: &Context) -> Option<PreparedPackageFetchCache>;

    fn fetch_dest(&self, context: &Context) -> PreparedPackageFetchDest;
}

pub(crate) struct PreparedPackageFetchCache {
    pub(crate) file_location_parent: PathBuf,
    pub(crate) file_location: PathBuf,

    pub(crate) symlink_location_diff: PathBuf,
    pub(crate) symlink_location_tmp: PathBuf,
    pub(crate) symlink_location: PathBuf,
}

#[expect(clippy::struct_field_names)]
pub(crate) struct PreparedPackageFetchDest {
    pub(crate) dir_location_parent_parent: PathBuf,
    pub(crate) dir_location_parent: PathBuf,
    pub(crate) dir_location: PathBuf,
}
