use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};
use base16ct::HexDisplay;
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use url::Url;

use super::{
    Packageable,
    PreparedPackageCache,
    PreparedPackageDest,
    PreparedPackageable,
    PreparedPackageableInner,
    RawPackageCache,
    RawPackageable,
    ResolvedPackageable,
};
use crate::context::{Context, ProjectDirs as _};

#[derive(Deserialize)]
pub(crate) struct RawCask {
    token: String,
    version: String,
    url: String,
    sha256: String,
}

impl Packageable for RawCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl RawPackageable for RawCask {
    fn version(&self) -> Cow<'_, str> {
        let version = &self.version;

        Cow::Borrowed(version)
    }

    fn cache(&self, context: &Context) -> RawPackageCache {
        let id = self.id();

        let file_name = format!("{id}.json");

        let cache_dir = context.homebrew_dirs.cache_dir();

        let file_location_parent = cache_dir.join("api").join("cask");

        let file_location = file_location_parent.join(file_name);

        RawPackageCache {
            file_location_parent,
            file_location,
        }
    }
}

pub(crate) struct ResolvedCask {
    token: String,
    version: String,
    url: String,
    sha256: String,
}

impl From<RawCask> for ResolvedCask {
    fn from(raw_cask: RawCask) -> Self {
        Self {
            token: raw_cask.token,
            version: raw_cask.version,
            url: raw_cask.url,
            sha256: raw_cask.sha256,
        }
    }
}

impl Packageable for ResolvedCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl ResolvedPackageable for ResolvedCask {
    fn version(&self) -> Cow<'_, str> {
        let version = &self.version;

        Cow::Borrowed(version)
    }
}

pub(crate) struct PreparedCask {
    token: String,
    version: String,
    url: String,
    sha256: String,
}

impl From<ResolvedCask> for PreparedCask {
    fn from(resolved_cask: ResolvedCask) -> Self {
        #[expect(resolving_to_items_shadowing_supertrait_items)]
        let version = resolved_cask.version().into_owned();

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
    pub(crate) fn url(&self) -> &str {
        &self.url
    }
}

pub(crate) struct FetchedCask {
    token: String,
    version: String,
    caskroom_dir: PathBuf,
}

impl From<(PreparedCask, PreparedPackageDest)> for FetchedCask {
    fn from((prepared_cask, dest): (PreparedCask, PreparedPackageDest)) -> Self {
        Self {
            token: prepared_cask.token,
            version: prepared_cask.version,
            caskroom_dir: dest.dir_location_grandparent,
        }
    }
}

impl Packageable for FetchedCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}
