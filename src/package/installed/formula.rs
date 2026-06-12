use super::{
    super::{Packageable, prepared::PreparedFormula},
    InstalledPackageable,
};

pub(crate) struct InstalledFormula {
    name: String,
    version: String,
    is_requested: bool,
}

impl From<PreparedFormula> for InstalledFormula {
    fn from(prepared_formula: PreparedFormula) -> Self {
        Self {
            name: prepared_formula.name,
            version: prepared_formula.version,
            is_requested: prepared_formula.is_requested,
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
