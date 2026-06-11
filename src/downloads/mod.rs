mod cask;
mod formula;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use base16ct::HexDisplay;
use sha2::{Digest as _, Sha256};
use tokio::{fs::File, io};
use tokio_util::io::InspectWriter;

use self::{cask::CaskDownload, formula::FormulaDownload};
use crate::{
    context::Context,
    ext::tokio::{fs::FileExt as _, path::PathExt as _},
    package::prepared::PreparedPackage,
    util::ArchiveFormat,
};

pub(crate) struct Download {
    pub(crate) archive_format: Option<ArchiveFormat>,
    pub(crate) link_path: PathBuf,
    pub(crate) file_path: PathBuf,
    pub(crate) is_verified: bool,
}

pub(crate) struct Downloads {
    formula_download: FormulaDownload,
    cask_download: CaskDownload,

    context: Arc<Context>,
}

impl Downloads {
    pub(crate) fn new(context: Arc<Context>) -> Self {
        Self {
            formula_download: FormulaDownload::new(Arc::clone(&context)),
            cask_download: CaskDownload::new(Arc::clone(&context)),

            context,
        }
    }

    pub(crate) async fn retrieve(
        &self,
        prepared_package: &PreparedPackage,
        expected_sha256: &str,
    ) -> anyhow::Result<Download> {
        let download = match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                self.formula_download
                    .retrieve(prepared_formula, expected_sha256)
                    .await?
            },
            PreparedPackage::Cask(prepared_cask) => {
                self.cask_download
                    .retrieve(prepared_cask, expected_sha256)
                    .await?
            },
        };

        Ok(download)
    }
}

trait Downloadable {
    type PreparedPackage;

    fn new(context: Arc<Context>) -> Self;

    fn archive_format(&self, link_path: &Path) -> anyhow::Result<Option<ArchiveFormat>>;

    async fn link_path_file_path(
        &self,
        prepared_package: &Self::PreparedPackage,
    ) -> anyhow::Result<(PathBuf, PathBuf)>;

    async fn sha256(&self, file_path: &Path) -> anyhow::Result<Option<String>> {
        let Some(mut file) = File::open_if_exists(file_path).await? else {
            return Ok(None);
        };

        let mut digest = Sha256::new();

        let mut sink = InspectWriter::new(io::sink(), |chunk| digest.update(chunk));

        io::copy(&mut file, &mut sink).await?;

        let sha256 = digest.finalize();
        let sha256 = HexDisplay(&sha256);
        let sha256 = format!("{sha256:x}");

        Ok(Some(sha256))
    }

    async fn verify(
        &self,
        link_path: &Path,
        file_path: &Path,
        file_sha256: &str,
        expected_sha256: &str,
    ) -> anyhow::Result<bool> {
        let is_file_exists = file_path.is_file_exists_nofollow().await?;

        let is_link_exists = link_path.is_link_exists_nofollow().await?;

        let is_link_valid =
            link_path.realpath_or_none().await? == file_path.realpath_or_none().await?;

        let is_sha256_equal = file_sha256 == expected_sha256;

        let is_verified = is_file_exists && is_link_exists && is_link_valid && is_sha256_equal;

        Ok(is_verified)
    }

    async fn retrieve(
        &self,
        prepared_package: &Self::PreparedPackage,
        expected_sha256: &str,
    ) -> anyhow::Result<Download> {
        let (link_path, file_path) = self.link_path_file_path(prepared_package).await?;

        let archive_format = self.archive_format(&link_path)?;

        let Some(file_sha256) = self.sha256(&file_path).await? else {
            let download = Download {
                archive_format,
                link_path,
                file_path,
                is_verified: false,
            };

            return Ok(download);
        };

        let is_verified = self
            .verify(&link_path, &file_path, &file_sha256, expected_sha256)
            .await?;

        let download = Download {
            archive_format,
            link_path,
            file_path,
            is_verified,
        };

        Ok(download)
    }
}
