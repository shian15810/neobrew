mod cask;
mod formula;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use base16ct::HexDisplay;
use sha2::{Digest as _, Sha256};
use tokio::{fs::File, io};
use tokio_util::io::InspectWriter;

use self::{cask::CaskCache, formula::FormulaCache};
use crate::{
    context::Context,
    ext::tokio::{fs::FileExt as _, path::PathExt as _},
    package::prepared::PreparedPackage,
    util::ArchiveFormat,
};

pub(crate) struct Caches {
    formula_cache: FormulaCache,
    cask_cache: CaskCache,

    context: Arc<Context>,
}

impl Caches {
    pub(crate) fn new(context: Arc<Context>) -> Self {
        let formula_cache = FormulaCache::new(Arc::clone(&context));

        let cask_cache = CaskCache::new(Arc::clone(&context));

        Self {
            formula_cache,
            cask_cache,

            context,
        }
    }

    pub(crate) async fn retrieve(
        &self,
        prepared_package: &PreparedPackage,
        expected_sha256: &str,
    ) -> Result<Cache> {
        let cache = match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                self.formula_cache
                    .retrieve(prepared_formula, expected_sha256)
                    .await?
            },
            PreparedPackage::Cask(prepared_cask) => {
                self.cask_cache
                    .retrieve(prepared_cask, expected_sha256)
                    .await?
            },
        };

        Ok(cache)
    }
}

trait Cacheable {
    type PreparedPackage;

    fn new(context: Arc<Context>) -> Self;

    fn archive_format(&self, symlink_path: &Path) -> Result<Option<ArchiveFormat>>;

    async fn symlink_file_paths(
        &self,
        prepared_package: &Self::PreparedPackage,
    ) -> Result<(PathBuf, PathBuf)>;

    async fn file_sha256(&self, file_path: &Path) -> Result<Option<String>> {
        let Some(mut file) = File::open_if_exists(file_path).await? else {
            return Ok(None);
        };

        let mut hasher = Sha256::new();

        let mut sink = InspectWriter::new(io::sink(), |bytes| hasher.update(bytes));

        io::copy(&mut file, &mut sink).await?;

        let file_sha256 = hasher.finalize();
        let file_sha256 = HexDisplay(&file_sha256);
        let file_sha256 = format!("{file_sha256:x}");

        Ok(Some(file_sha256))
    }

    async fn is_valid(
        &self,
        symlink_path: &Path,
        file_path: &Path,
        file_sha256: &str,
        expected_sha256: &str,
    ) -> Result<bool> {
        let is_valid = symlink_path.is_symlink_exists_nofollow().await?
            && file_path.is_file_exists_nofollow().await?
            && symlink_path.realpath_or_none().await? == file_path.realpath_or_none().await?
            && file_sha256 == expected_sha256;

        Ok(is_valid)
    }

    async fn retrieve(
        &self,
        prepared_package: &Self::PreparedPackage,
        expected_sha256: &str,
    ) -> Result<Cache> {
        let (symlink_path, file_path) = self.symlink_file_paths(prepared_package).await?;

        let archive_format = self.archive_format(&symlink_path)?;

        let Some(file_sha256) = self.file_sha256(&file_path).await? else {
            let cache = Cache {
                archive_format,
                symlink_path,
                file_path,
                is_valid: false,
            };

            return Ok(cache);
        };

        let is_valid = self
            .is_valid(&symlink_path, &file_path, &file_sha256, expected_sha256)
            .await?;

        let cache = Cache {
            archive_format,
            symlink_path,
            file_path,
            is_valid,
        };

        Ok(cache)
    }
}

pub(crate) struct Cache {
    pub(crate) archive_format: Option<ArchiveFormat>,
    pub(crate) symlink_path: PathBuf,
    pub(crate) file_path: PathBuf,
    pub(crate) is_valid: bool,
}
