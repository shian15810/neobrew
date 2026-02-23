use serde::Deserialize;

use super::Packageable;

#[derive(Deserialize)]
pub struct RawCask {
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
}

pub struct ResolvedCask {
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
}
