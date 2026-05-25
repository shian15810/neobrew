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
    package::{
        Packageable as _,
        prepared::{PreparedCask, PreparedPackageable as _},
    },
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

        let mut segment = url.path_segments().context("Invalid URL")?;
        let segment = segment.next_back().context("Empty URL path segments")?;

        let path = Path::new(segment);

        let extension = path.extension().context("Invalid file name")?;
        let extension = extension.to_str().context("Invalid file extension")?;

        let symlink_name = format!("{segment}--{version}.{extension}");

        let file_name = format!("{url_hash}--{segment}");

        let symlink_path = cache_dir_path.join("Cask").join(symlink_name);

        let file_path = cache_dir_path.join("Cask/downloads").join(file_name);

        let symlink_file_paths = (symlink_path, file_path);

        Ok(symlink_file_paths)
    }
}
