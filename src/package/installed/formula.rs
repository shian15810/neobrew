use super::{
    super::{Packageable, streamed::StreamedFormula},
    InstalledPackageable,
};

pub(crate) struct InstalledFormula {
    name: String,
    version: String,
    version_revision: String,
    is_requested: bool,
}

impl From<StreamedFormula> for InstalledFormula {
    fn from(streamed_formula: StreamedFormula) -> Self {
        Self {
            name: streamed_formula.name,
            version: streamed_formula.version,
            version_revision: streamed_formula.version_revision,
            is_requested: streamed_formula.is_requested,
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
