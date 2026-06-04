mod cask;
mod formula;

use enum_dispatch::enum_dispatch;

pub(crate) use self::{cask::PipelinedCask, formula::PipelinedFormula};
use super::{Packageable, prepared::PreparedPackage};

#[expect(clippy::large_enum_variant)]
#[enum_dispatch]
pub(crate) enum PipelinedPackage {
    Formula(PipelinedFormula),
    Cask(PipelinedCask),
}

impl From<PreparedPackage> for PipelinedPackage {
    fn from(prepared_package: PreparedPackage) -> Self {
        match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                let pipelined_formula = PipelinedFormula::from(prepared_formula);

                Self::Formula(pipelined_formula)
            },
            PreparedPackage::Cask(prepared_cask) => {
                let pipelined_cask = PipelinedCask::from(prepared_cask);

                Self::Cask(pipelined_cask)
            },
        }
    }
}

#[enum_dispatch(PipelinedPackage)]
trait PipelinedPackageable: Packageable {}
