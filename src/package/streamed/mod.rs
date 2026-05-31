mod cask;
mod formula;

use enum_dispatch::enum_dispatch;

pub(crate) use self::{cask::StreamedCask, formula::StreamedFormula};
use super::{Packageable, prepared::PreparedPackage};

#[enum_dispatch]
pub(crate) enum StreamedPackage {
    Formula(StreamedFormula),
    Cask(StreamedCask),
}

impl From<PreparedPackage> for StreamedPackage {
    fn from(prepared_package: PreparedPackage) -> Self {
        match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                let streamed_formula = StreamedFormula::from(prepared_formula);

                Self::Formula(streamed_formula)
            },
            PreparedPackage::Cask(prepared_cask) => {
                let streamed_cask = StreamedCask::from(prepared_cask);

                Self::Cask(streamed_cask)
            },
        }
    }
}

#[enum_dispatch(StreamedPackage)]
trait StreamedPackageable: Packageable {}
