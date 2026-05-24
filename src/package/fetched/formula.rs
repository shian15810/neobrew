use std::sync::Arc;

use anyhow::Result;

use super::super::{Packageable, prepared::PreparedFormula, raw::BottleStableFileCellar};
use crate::{
    context::Context,
    utils::{Linker, Relocation},
};

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
    pub(crate) async fn relocate(
        &self,
        relocation: Arc<Relocation>,
        context: &Context,
    ) -> Result<()> {
        if self.bottle_file_cellar == BottleStableFileCellar::AnySkipRelocation {
            return Ok(());
        }

        let keg_dir_path = context.homebrew_dirs.keg_dir(self.id(), self.version());

        relocation.patch_keg(&keg_dir_path).await?;

        Ok(())
    }

    pub(crate) async fn link(&self, linker: &Linker) -> Result<()> {
        linker.link_opt(self).await?;

        linker.link_keg(self).await?;

        Ok(())
    }
}
