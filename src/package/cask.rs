use std::path::Path;

use anyhow::{Context as _, Result};
use base16ct::HexDisplay;
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use url::Url;

use super::{
    Packageable,
    PreparedPackageCache,
    PreparedPackageable,
    PreparedPackageableInner,
    RawPackageCache,
    RawPackageable,
};
use crate::context::{Context, ProjectDirs as _};

#[derive(Deserialize)]
pub(crate) struct RawCask {
    token: String,
    name: Vec<String>,
    url: String,
    version: String,
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
    name: Vec<String>,
    url: String,
    version: String,
    sha256: String,
}

impl From<RawCask> for ResolvedCask {
    fn from(raw_cask: RawCask) -> Self {
        Self {
            token: raw_cask.token,
            name: raw_cask.name,
            url: raw_cask.url,
            version: raw_cask.version,
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

pub(crate) struct PreparedCask {
    token: String,
    name: Vec<String>,
    url: String,
    version: String,
    sha256: String,
}

impl From<ResolvedCask> for PreparedCask {
    fn from(resolved_cask: ResolvedCask) -> Self {
        Self {
            token: resolved_cask.token,
            name: resolved_cask.name,
            url: resolved_cask.url,
            version: resolved_cask.version,
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
    name: Vec<String>,
    url: String,
    version: String,
    sha256: String,
}

impl From<PreparedCask> for FetchedCask {
    fn from(prepared_cask: PreparedCask) -> Self {
        Self {
            token: prepared_cask.token,
            name: prepared_cask.name,
            url: prepared_cask.url,
            version: prepared_cask.version,
            sha256: prepared_cask.sha256,
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
