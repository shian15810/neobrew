mod cask;
mod formula;

use enum_dispatch::enum_dispatch;

pub(crate) use self::{cask::FetchedCask, formula::FetchedFormula};
use super::{
    Packageable,
    prepared::{PreparedPackage, PreparedPackageDest},
};

#[enum_dispatch]
pub(crate) enum FetchedPackage {
    Formula(FetchedFormula),
    Cask(FetchedCask),
}

impl From<(PreparedPackage, PreparedPackageDest)> for FetchedPackage {
    fn from((prepared_package, dest): (PreparedPackage, PreparedPackageDest)) -> Self {
        match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                let fetched_formula = FetchedFormula::from((prepared_formula, dest));

                Self::Formula(fetched_formula)
            },
            PreparedPackage::Cask(prepared_cask) => {
                let fetched_cask = FetchedCask::from((prepared_cask, dest));

                Self::Cask(fetched_cask)
            },
        }
    }
}

#[enum_dispatch(FetchedPackage)]
trait FetchedPackageable: Packageable {}
