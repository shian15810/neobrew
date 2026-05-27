use super::super::{Packageable, prepared::PreparedFormula, raw::BottleStableFileCellar};

pub(crate) struct FetchedFormula {
    name: String,
    version_revision: String,
    bottle_file_cellar: BottleStableFileCellar,
    keg_only: bool,
}

impl From<PreparedFormula> for FetchedFormula {
    fn from(prepared_formula: PreparedFormula) -> Self {
        Self {
            name: prepared_formula.name,
            version_revision: prepared_formula.version_revision,
            bottle_file_cellar: prepared_formula.bottle_file.cellar,
            keg_only: prepared_formula.keg_only,
        }
    }
}

impl Packageable for FetchedFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version_revision
    }
}

impl FetchedFormula {
    pub(crate) fn should_relocate(&self) -> bool {
        self.bottle_file_cellar != BottleStableFileCellar::AnySkipRelocation
    }

    pub(crate) fn should_link_keg(&self) -> bool {
        !self.keg_only
    }
}
