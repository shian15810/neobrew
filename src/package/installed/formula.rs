use super::{
    super::{PackageExt, prepared::formula::PreparedFormula},
    InstalledPackageExt,
};

pub(crate) struct InstalledFormula {
    id: String,
    version: String,
    is_requested: bool,
}

impl From<PreparedFormula> for InstalledFormula {
    fn from(prepared_formula: PreparedFormula) -> Self {
        Self {
            id: prepared_formula.id,
            version: prepared_formula.version,
            is_requested: prepared_formula.is_requested,
        }
    }
}

impl PackageExt for InstalledFormula {
    fn id(&self) -> &str {
        &self.id
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl InstalledPackageExt for InstalledFormula {}
