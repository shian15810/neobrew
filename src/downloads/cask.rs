use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context as _;
use base16ct::HexDisplay;
use sha2::{Digest as _, Sha256};
use url::Url;

use super::Downloadable;
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    ext::std::path::PathExt as _,
    package::{
        Packageable as _,
        prepared::{PreparedCask, PreparedPackageable as _},
    },
    util::ArchiveFormat,
};

pub(super) struct CaskDownload {
    context: Arc<Context>,
}

impl Downloadable for CaskDownload {
    type PreparedPackage = PreparedCask;

    fn new(context: Arc<Context>) -> Self {
        Self {
            context,
        }
    }

    fn archive_format(&self, symlink_path: &Path) -> anyhow::Result<Option<ArchiveFormat>> {
        let archive_format = match ArchiveFormat::try_from(symlink_path) {
            Ok(archive_format) => archive_format,
            Err(Some(err)) => return Err(err),
            Err(None) => return Ok(None),
        };

        Ok(Some(archive_format))
    }

    async fn symlink_file_paths(
        &self,
        prepared_package: &Self::PreparedPackage,
    ) -> anyhow::Result<(PathBuf, PathBuf)> {
        let prepared_cask = prepared_package;

        let dir_path = self.context.homebrew_dirs.cache_dir();

        let version = prepared_cask.version();

        let url = prepared_cask.download_url();

        let resp = self.context.client.get(url).send().await?;
        let resp = resp.error_for_status()?;

        let url = resp.url().as_str();

        let url_hash = Sha256::digest(url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let url = Url::parse(url)?;

        let mut name = url.path_segments().context("Invalid URL")?;
        let name = name.next_back().context("Empty path segments")?;

        let path = Path::new(name);

        let compound_extension = path.compound_extension();

        let symlink_name = match compound_extension {
            None => format!("{name}--{version}"),
            Some(compound_extension) => {
                let compound_extension = compound_extension
                    .to_str()
                    .context("Invalid compound extension")?;

                format!("{name}--{version}.{compound_extension}")
            },
        };

        let file_name = format!("{url_hash}--{name}");

        let symlink_path = dir_path.join("Cask").join(symlink_name);

        let file_path = dir_path.join("downloads").join(file_name);

        let symlink_file_paths = (symlink_path, file_path);

        Ok(symlink_file_paths)
    }
}
