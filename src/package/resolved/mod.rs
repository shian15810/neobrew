pub(crate) mod cask;
pub(crate) mod formula;

use std::sync::Arc;

use either::{Left, Right};
use enum_dispatch::enum_dispatch;

use self::{cask::ResolvedCask, formula::ResolvedFormula};
use super::PackageExt;

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
                let casks = cask.iter().map(Self::Cask);

                Right(casks)
            },
        }
    }
}

#[enum_dispatch(ResolvedPackage)]
pub(crate) trait ResolvedPackageExt: PackageExt {
    fn set_is_compatible(&self, is_compatible: bool);

    fn set_is_requested(&self, is_requested: bool);
}

impl<ResolvedPackage: ResolvedPackageExt> ResolvedPackageExt for Arc<ResolvedPackage> {
    fn set_is_compatible(&self, is_compatible: bool) {
        ResolvedPackage::set_is_compatible(self, is_compatible);
    }

    fn set_is_requested(&self, is_requested: bool) {
        ResolvedPackage::set_is_requested(self, is_requested);
    }
}

trait ResolvedPackageExtIter: ResolvedPackageExt {
    fn iter(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> + use<Self>;
}
