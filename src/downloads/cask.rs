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

    async fn symlink_path_file_path(
        &self,
        prepared_package: &PreparedCask,
    ) -> anyhow::Result<(PathBuf, PathBuf)> {
        let prepared_cask = prepared_package;

        let version = prepared_cask.version();

        let url = prepared_cask.download_url();

        let resp = self.context.client.get(url).send().await?;
        let resp = resp.error_for_status()?;

        let url = resp.url().as_str();

        let url_hash = Sha256::digest(url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let url = Url::parse(url)?;

        let mut url_name = url.path_segments().context("Invalid URL")?;
        let url_name = url_name.next_back().context("Empty path segments")?;

        let url_path = Path::new(url_name);

        let url_compound_extension = url_path.compound_extension();

        let symlink_name = match url_compound_extension {
            Some(url_compound_extension) => {
                let url_compound_extension = url_compound_extension
                    .to_str()
                    .context("Invalid compound extension")?;

                format!("{url_name}--{version}.{url_compound_extension}")
            },
            None => format!("{url_name}--{version}"),
        };

        let file_name = format!("{url_hash}--{url_name}");

        let cache_dir_path = self.context.homebrew_dirs.cache_dir();

        let symlink_path = cache_dir_path.join("Cask").join(symlink_name);

        let file_path = cache_dir_path.join("downloads").join(file_name);

        let symlink_path_file_path = (symlink_path, file_path);

        Ok(symlink_path_file_path)
    }
}
