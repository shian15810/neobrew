mod cask;
mod formula;

use std::sync::Arc;

use either::{Left, Right};
use enum_dispatch::enum_dispatch;

pub(crate) use self::{cask::ResolvedCask, formula::ResolvedFormula};
use super::Packageable;

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
pub(super) trait ResolvedPackageable: Packageable {}

impl<ResolvedPackage: ResolvedPackageable> ResolvedPackageable for Arc<ResolvedPackage> {}

trait ResolvedPackageableIter: ResolvedPackageable {
    fn iter(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> + use<Self>;
}
