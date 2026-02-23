use std::{iter, sync::Arc};

use enum_dispatch::enum_dispatch;
use itertools::Either;

use self::{
    cask::{RawCask, ResolvedCask},
    formula::{RawFormula, ResolvedFormula},
};

pub mod cask;
pub mod formula;

#[enum_dispatch]
enum Package {
    Raw(RawPackage),
    Resolved(ResolvedPackage),
}

#[enum_dispatch]
pub enum RawPackage {
    Formula(RawFormula),
    Cask(RawCask),
}

#[enum_dispatch]
pub enum ResolvedPackage {
    Formula(Arc<ResolvedFormula>),
    Cask(Arc<ResolvedCask>),
}

impl ResolvedPackage {
    pub fn iter(&self) -> impl Iterator<Item = Self> + use<> {
        match self {
            Self::Formula(formula) => {
                let formulae = formula.iter().map(Self::Formula);

                Either::Left(formulae)
            },
            Self::Cask(cask) => {
                let casks = iter::once(Arc::clone(cask)).map(Self::Cask);

                Either::Right(casks)
            },
        }
    }
}

#[enum_dispatch(Package, RawPackage, ResolvedPackage)]
pub trait Packageable {
    fn id(&self) -> &str;
}

impl<Package: Packageable> Packageable for Arc<Package> {
    fn id(&self) -> &str {
        let this = &**self;

        this.id()
    }
}
