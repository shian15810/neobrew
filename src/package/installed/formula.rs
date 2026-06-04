use super::{
    super::{Packageable, pipelined::PipelinedFormula},
    InstalledPackageable,
};

pub(crate) struct InstalledFormula {
    name: String,
    version: String,
    is_requested: bool,
}

impl From<PipelinedFormula> for InstalledFormula {
    fn from(pipelined_formula: PipelinedFormula) -> Self {
        Self {
            name: pipelined_formula.name,
            version: pipelined_formula.version,
            is_requested: pipelined_formula.is_requested,
        }
    }
}

impl Packageable for InstalledFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl InstalledPackageable for InstalledFormula {}
