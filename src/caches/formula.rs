use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use base16ct::HexDisplay;
use sha2::{Digest as _, Sha256};

use super::Cacheable;
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    package::{
        Packageable as _,
        prepared::{PreparedFormula, PreparedPackageable as _},
    },
};

pub(super) struct FormulaCache {
    context: Arc<Context>,
}

impl Cacheable for FormulaCache {
    type PreparedPackage = PreparedFormula;

    fn new(context: Arc<Context>) -> Self {
        Self {
            context,
        }
    }

    fn symlink_file_paths(
        &self,
        prepared_package: &Self::PreparedPackage,
    ) -> Result<(PathBuf, PathBuf)> {
        let prepared_formula = prepared_package;

        let cache_dir_path = self.context.homebrew_dirs.cache_dir();

        let id = prepared_formula.id();

        let version = prepared_formula.version();

        let bottle_rebuild = prepared_formula.bottle_rebuild();

        let bottle_tag = prepared_formula.bottle_tag();

        let url = prepared_formula.cache_url();

        let url_hash = Sha256::digest(url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let symlink_name = format!("{id}--{version}");

        let file_name = format!("{url_hash}--{id}--{version}.{bottle_tag}.bottle");
        let file_name = match bottle_rebuild {
            0 => format!("{file_name}.tar.gz"),
            bottle_rebuild => format!("{file_name}.{bottle_rebuild}.tar.gz"),
        };

        let symlink_path = cache_dir_path.join(symlink_name);

        let file_path = cache_dir_path.join("downloads").join(file_name);

        let symlink_file_paths = (symlink_path, file_path);

        Ok(symlink_file_paths)
    }
}
