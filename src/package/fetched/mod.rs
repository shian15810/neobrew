mod cask;
mod formula;

use enum_dispatch::enum_dispatch;

pub(crate) use self::{cask::FetchedCask, formula::FetchedFormula};
use super::{Packageable, prepared::PreparedPackage};

#[enum_dispatch]
pub(crate) enum FetchedPackage {
    Formula(FetchedFormula),
    Cask(FetchedCask),
}

impl From<PreparedPackage> for FetchedPackage {
    fn from(prepared_package: PreparedPackage) -> Self {
        match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                let fetched_formula = FetchedFormula::from(prepared_formula);

                Self::Formula(fetched_formula)
            },
            PreparedPackage::Cask(prepared_cask) => {
                let fetched_cask = FetchedCask::from(prepared_cask);

                Self::Cask(fetched_cask)
            },
        }
    }
}

#[enum_dispatch(FetchedPackage)]
trait FetchedPackageable: Packageable {}
