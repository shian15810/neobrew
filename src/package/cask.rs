use std::path::Path;

use base16ct::HexDisplay;
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use url::Url;

use super::{Packageable, ResolvedPackageCache, ResolvedPackageable};

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

pub(crate) struct ResolvedCask {
    token: String,
    name: Vec<String>,
    url: String,
    version: String,
    sha256: String,
}

impl From<RawCask> for ResolvedCask {
    fn from(raw: RawCask) -> Self {
        Self {
            token: raw.token,
            name: raw.name,
            url: raw.url,
            version: raw.version,
            sha256: raw.sha256,
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
    fn cache(&self) -> Option<ResolvedPackageCache> {
        let version = &self.version();

        let url = Url::parse(&self.url).ok()?;

        let name = url
            .path_segments()
            .and_then(|mut path_segments| path_segments.next_back())?;

        let extension = Path::new(name).extension()?.to_str()?;

        let url_hash = format!("{:x}", HexDisplay(&Sha256::digest(&self.url)));

        let symlink_name = format!("{name}--{version}.{extension}");

        let file_name = format!("{url_hash}--{name}");

        let cache = ResolvedPackageCache {
            file_name,
            symlink_name,
        };

        Some(cache)
    }

    fn sha256(&self) -> Option<&str> {
        let sha256 = &self.sha256;

        Some(sha256)
    }
}

impl ResolvedCask {
    pub(crate) fn url(&self) -> &str {
        &self.url
    }
}
