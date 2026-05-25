mod cask;
mod formula;

use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use base16ct::HexDisplay;
use digest_io::IoWrapper;
use sha2::{Digest as _, Sha256};
use tokio::{fs::File, task};
use tokio_util::task::AbortOnDropHandle;

use self::{cask::CaskCache, formula::FormulaCache};
use crate::{
    context::Context,
    ext::tokio::{fs::FileExt as _, path::PathExt as _},
    package::prepared::PreparedPackage,
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

    fn symlink_file_paths(
        &self,
        prepared_package: &Self::PreparedPackage,
    ) -> Result<(PathBuf, PathBuf)>;

    async fn file_sha256(&self, file_path: &Path) -> Result<Option<String>> {
        let Some(file) = File::open_if_exists(file_path).await? else {
            return Ok(None);
        };

        let mut file = file.into_std().await;

        let mut hasher = IoWrapper(Sha256::new());

        let handle = task::spawn_blocking(move || {
            io::copy(&mut file, &mut hasher)?;

            anyhow::Ok(Some(hasher))
        });
        let handle = AbortOnDropHandle::new(handle);

        let Some(hasher) = handle.await?? else {
            return Ok(None);
        };

        let file_sha256 = hasher.0.finalize();
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
        let is_cache_valid = symlink_path.is_symlink_exists_nofollow().await?
            && file_path.is_file_exists_nofollow().await?
            && symlink_path.canonicalize()? == file_path.canonicalize()?
            && file_sha256 == expected_sha256;

        Ok(is_cache_valid)
    }

    async fn retrieve(
        &self,
        prepared_package: &Self::PreparedPackage,
        expected_sha256: &str,
    ) -> Result<Cache> {
        let (symlink_path, file_path) = self.symlink_file_paths(prepared_package)?;

        let Some(file_sha256) = self.file_sha256(&file_path).await? else {
            let cache = Cache {
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
            symlink_path,
            file_path,
            is_valid,
        };

        Ok(cache)
    }
}

pub(crate) struct Cache {
    pub(crate) symlink_path: PathBuf,
    pub(crate) file_path: PathBuf,
    pub(crate) is_valid: bool,
}
