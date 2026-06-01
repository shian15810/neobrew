use std::path::Path;

use super::{
    super::{Packageable, prepared::PreparedFormula, raw::BottleStableFileCellar},
    StreamedPackageable,
};

pub(crate) struct StreamedFormula {
    pub(in super::super) name: String,
    pub(in super::super) version: String,
    pub(in super::super) version_revision: String,
    bottle_cellar: BottleStableFileCellar,
    keg_only: bool,
    pub(in super::super) is_requested: bool,
}

impl From<PreparedFormula> for StreamedFormula {
    fn from(prepared_formula: PreparedFormula) -> Self {
        Self {
            name: prepared_formula.name,
            version: prepared_formula.version,
            version_revision: prepared_formula.version_revision,
            bottle_cellar: prepared_formula.bottle_cellar,
            keg_only: prepared_formula.keg_only,
            is_requested: prepared_formula.is_requested,
        }
    }
}

impl Packageable for StreamedFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl StreamedPackageable for StreamedFormula {}

impl StreamedFormula {
    pub(crate) fn version_revision(&self) -> &str {
        &self.version_revision
    }

    pub(crate) fn should_relocate(&self, cellar_dir_path: &Path) -> bool {
        match &self.bottle_cellar {
            BottleStableFileCellar::Any => true,
            BottleStableFileCellar::AnySkipRelocation => false,
            BottleStableFileCellar::Path(path) => path == cellar_dir_path,
        }
    }

    pub(crate) fn should_link_keg(&self) -> bool {
        !self.keg_only
    }
}
