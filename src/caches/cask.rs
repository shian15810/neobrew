use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context as _, Result};
use base16ct::HexDisplay;
use sha2::{Digest as _, Sha256};
use url::Url;

use super::Cacheable;
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    ext::std::path::PathExt as _,
    package::{
        Packageable as _,
        prepared::{PreparedCask, PreparedPackageable as _},
    },
    util::ArchiveFormat,
};

pub(super) struct CaskCache {
    context: Arc<Context>,
}

impl Cacheable for CaskCache {
    type PreparedPackage = PreparedCask;

    fn new(context: Arc<Context>) -> Self {
        Self {
            context,
        }
    }

    fn archive_format(&self, symlink_path: &Path) -> Result<Option<ArchiveFormat>> {
        let archive_format = match ArchiveFormat::try_from(symlink_path) {
            Ok(archive_format) => archive_format,
            Err(Some(err)) => return Err(err),
            Err(None) => return Ok(None),
        };

        Ok(Some(archive_format))
    }

    fn symlink_file_paths(
        &self,
        prepared_package: &Self::PreparedPackage,
    ) -> Result<(PathBuf, PathBuf)> {
        let prepared_cask = prepared_package;

        let cache_dir_path = self.context.homebrew_dirs.cache_dir();

        let version = prepared_cask.version();

        let url = prepared_cask.cache_url();

        let url_hash = Sha256::digest(url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let url = Url::parse(url)?;

        let mut name = url.path_segments().context("Invalid URL")?;
        let name = name.next_back().context("Empty URL path segments")?;

        let path = Path::new(name);

        let compound_extension = path
            .compound_extension()
            .context("Invalid file path name")?;
        let compound_extension = compound_extension
            .to_str()
            .context("Invalid file compound extension")?;

        let symlink_name = format!("{name}--{version}.{compound_extension}");

        let file_name = format!("{url_hash}--{name}");

        let symlink_path = cache_dir_path.join("Cask").join(symlink_name);

        let file_path = cache_dir_path.join("Cask/downloads").join(file_name);

        let symlink_file_paths = (symlink_path, file_path);

        Ok(symlink_file_paths)
    }
}
