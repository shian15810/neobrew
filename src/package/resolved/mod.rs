mod cask;
mod formula;

use std::{borrow::Cow, sync::Arc};

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

#[cfg_attr(debug_assertions, expect(shadowing_supertrait_items))]
#[enum_dispatch(ResolvedPackage)]
pub(super) trait ResolvedPackageable: Packageable {
    fn version(&self) -> Cow<'_, str>;
}

impl<ResolvedPackage: ResolvedPackageable> ResolvedPackageable for Arc<ResolvedPackage> {
    fn version(&self) -> Cow<'_, str> {
        <ResolvedPackage as ResolvedPackageable>::version(self)
    }
}

trait ResolvedPackageableIter: ResolvedPackageable {
    fn iter(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> + use<Self>;
}
