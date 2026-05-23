use anyhow::Result;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;
use walkdir::WalkDir;

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
    pub(crate) async fn relocate(&self, context: &Context) -> Result<()> {
        if self.bottle_file_cellar == BottleStableFileCellar::AnySkipRelocation {
            return Ok(());
        }

        let relocation = Relocation::from(&context.homebrew_dirs);

        let keg_dir_path = context.homebrew_dirs.keg_dir(self.id(), self.version());

        let handle = task::spawn_blocking(move || {
            for entry in WalkDir::new(keg_dir_path) {
                let entry = entry?;

                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                relocation.patch_file(path)?;
            }

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }

    pub(crate) async fn link(&self, linker: &Linker) -> Result<()> {
        linker.link_opt(self).await?;

        linker.link_keg(self).await?;

        Ok(())
    }
}
