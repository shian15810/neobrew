use std::path::PathBuf;

use anyhow::Result;
use walkdir::WalkDir;

use super::super::{
    Packageable,
    prepared::{PreparedFormula, PreparedPackageDest},
};
use crate::context::Context;

pub(crate) struct FetchedFormula {
    name: String,
    version: String,
    prefix_dir: PathBuf,
    cellar_dir: PathBuf,
    rack_dir: PathBuf,
    keg_dir: PathBuf,
}

impl From<(PreparedFormula, PreparedPackageDest)> for FetchedFormula {
    fn from((prepared_formula, dest): (PreparedFormula, PreparedPackageDest)) -> Self {
        Self {
            name: prepared_formula.name,
            version: prepared_formula.version,
            prefix_dir: dest.dir_location_greatgrandparent,
            cellar_dir: dest.dir_location_grandparent,
            rack_dir: dest.dir_location_parent,
            keg_dir: dest.dir_location,
        }
    }
}

impl Packageable for FetchedFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl FetchedFormula {
    #[cfg(target_os = "macos")]
    pub(crate) fn relocate_keg(&self, context: &Context) -> Result<()> {
        use crate::os::macos::{Codesign, Relocation};

        let relocation = Relocation::from(&context.homebrew_dirs);

        for entry in WalkDir::new(&self.keg_dir) {
            let entry = entry?;

            let path = entry.path();

            relocation.patch_file(path)?;

            Codesign::sign_in_place(path)?;
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn relocate_keg(&self, context: &Context) -> Result<()> {
        use crate::os::linux::Relocation;

        let relocation = Relocation::from(&context.homebrew_dirs);

        for entry in WalkDir::new(&self.keg_dir) {
            let entry = entry?;

            let path = entry.path();

            relocation.patch_file(path)?;
        }

        Ok(())
    }
}
