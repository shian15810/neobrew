use std::path::Path;

use base16ct::HexDisplay;
use pathdiff::diff_paths;
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use url::Url;

use super::{
    Packageable,
    PreparedPackageFetchCache,
    PreparedPackageFetchDest,
    PreparedPackageable,
    RawPackageJsonCache,
    RawPackageable,
};
use crate::Context;

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
    fn json_cache(&self, context: &Context) -> RawPackageJsonCache {
        let id = self.id();

        let file_name = format!("{id}.json");

        let cache_dir = cfg_select! {
            debug_assertions => context.neobrew_dirs.cache_dir(),
            _ => context.homebrew_dirs.cache_dir(),
        };

        let file_location_parent = cache_dir.join("api").join("cask");

        let file_location = file_location_parent.join(file_name);

        RawPackageJsonCache {
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
    fn fetch_sha256(&self) -> &str {
        &self.sha256
    }

    fn fetch_cache(&self, context: &Context) -> Option<PreparedPackageFetchCache> {
        let version = &self.version();

        let url = Url::parse(&self.url).ok()?;

        let mut name = url.path_segments()?;
        let name = name.next_back()?;

        let path = Path::new(name);

        let extension = path.extension()?;
        let extension = extension.to_str()?;

        let url_hash = Sha256::digest(&self.url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let symlink_name = format!("{name}--{version}.{extension}");

        let file_name = format!("{url_hash}--{name}");

        let cache_dir = cfg_select! {
            debug_assertions => context.neobrew_dirs.cache_dir(),
            _ => context.homebrew_dirs.cache_dir(),
        };

        let symlink_location_parent = cache_dir.join("Cask");

        let file_location_parent = symlink_location_parent.join("downloads");

        let file_location = file_location_parent.join(file_name);

        let symlink_location_diff = diff_paths(&file_location, &symlink_location_parent)?;

        let symlink_location = symlink_location_parent.join(symlink_name);

        let symlink_location_tmp = symlink_location.with_extension("tmp");

        let cache = PreparedPackageFetchCache {
            file_location_parent,
            file_location,

            symlink_location_diff,
            symlink_location_tmp,
            symlink_location,
        };

        Some(cache)
    }

    fn fetch_dest(&self, context: &Context) -> PreparedPackageFetchDest {
        let id = self.id();

        let version = self.version();

        let caskroom_dir = cfg_select! {
            debug_assertions => context.neobrew_dirs.caskroom_dir(),
            _ => context.homebrew_dirs.caskroom_dir(),
        };

        let dir_location_parent_parent = caskroom_dir;

        let dir_location_parent = dir_location_parent_parent.join(id);

        let dir_location = dir_location_parent.join(version);

        PreparedPackageFetchDest {
            dir_location_parent_parent,
            dir_location_parent,
            dir_location,
        }
    }
}

impl PreparedCask {
    pub(crate) fn fetch_url(&self) -> &str {
        &self.url
    }
}
