use std::path::Path;

use anyhow::{Context as _, Result};
use base16ct::HexDisplay;
use sha2::{Digest as _, Sha256};
use url::Url;

use super::{
    super::{
        Packageable,
        resolved::{ResolvedCask, ResolvedPackageable as _},
    },
    PreparedPackageCache,
    PreparedPackageable,
    PreparedPackageableInner,
};
use crate::context::{Context, dirs::ProjectDirs as _};

pub(crate) struct PreparedCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    url: String,
    sha256: String,
}

impl From<ResolvedCask> for PreparedCask {
    fn from(resolved_cask: ResolvedCask) -> Self {
        #[cfg(debug_assertions)]
        #[cfg_attr(
            debug_assertions,
            expect(resolving_to_items_shadowing_supertrait_items)
        )]
        let version = resolved_cask.version().into_owned();

        #[cfg(not(debug_assertions))]
        let version = ResolvedPackageable::version(&resolved_cask).into_owned();

        Self {
            token: resolved_cask.token,
            version,
            url: resolved_cask.url,
            sha256: resolved_cask.sha256,
        }
    }
}

impl Packageable for PreparedCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl PreparedPackageable for PreparedCask {
    async fn cache(&self, context: &Context) -> Result<PreparedPackageCache> {
        let version = self.version();

        let url = Url::parse(&self.url)?;

        let mut segment = url.path_segments().context("Invalid URL")?;
        let segment = segment.next_back().context("Empty URL path segments")?;

        let path = Path::new(segment);

        let extension = path.extension().context("Invalid file name")?;
        let extension = extension.to_str().context("Invalid file extension")?;

        let url_hash = Sha256::digest(&self.url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let symlink_name = format!("{segment}--{version}.{extension}");

        let file_name = format!("{url_hash}--{segment}");

        let cache_dir = context.homebrew_dirs.cache_dir();

        let symlink_location_parent = cache_dir.join("Cask");

        let cache = self.cache_inner(&file_name, &symlink_name, symlink_location_parent);

        Ok(cache)
    }

    fn sha256(&self) -> &str {
        &self.sha256
    }
}

impl PreparedPackageableInner for PreparedCask {}

impl PreparedCask {
    pub(super) fn url(&self) -> &str {
        &self.url
    }
}
