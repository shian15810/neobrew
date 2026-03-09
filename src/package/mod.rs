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
}

impl<Package: Packageable> Packageable for Arc<Package> {
    fn id(&self) -> &str {
        #[allow(clippy::use_self)]
        let this = Arc::as_ref(self);

        this.id()
    }
}
