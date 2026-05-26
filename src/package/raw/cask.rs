use std::borrow::Cow;

use serde::Deserialize;
use serde_json::{Map, Value};

use super::{super::Packageable, RawPackageable};

#[derive(Deserialize)]
pub(crate) struct RawCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
    pub(in super::super) artifacts: Vec<Map<String, Value>>,
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
}
