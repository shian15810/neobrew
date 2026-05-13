use std::{iter, sync::Arc};

use enum_dispatch::enum_dispatch;
use itertools::Either;

use self::{
    cask::{RawCask, ResolvedCask},
    formula::{RawFormula, ResolvedFormula},
};

pub(crate) mod cask;
pub(crate) mod formula;

#[enum_dispatch]
enum Package {
    Raw(RawPackage),
    Resolved(ResolvedPackage),
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

#[enum_dispatch(Package, RawPackage, ResolvedPackage)]
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

#[enum_dispatch(ResolvedPackage)]
pub(crate) trait ResolvedPackageable {
    fn cache(&self) -> Option<ResolvedPackageCache>;

    fn sha256(&self) -> Option<&str>;
}

impl<ResolvedPackage: ResolvedPackageable> ResolvedPackageable for Arc<ResolvedPackage> {
    fn cache(&self) -> Option<ResolvedPackageCache> {
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        let cache = this.cache()?;

        Some(cache)
    }

    fn sha256(&self) -> Option<&str> {
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        let sha256 = this.sha256()?;

        Some(sha256)
    }
}

pub(crate) struct ResolvedPackageCache {
    pub(crate) file_name: String,
    pub(crate) symlink_name: String,
}
