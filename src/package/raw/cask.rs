use std::{borrow::Cow, path::PathBuf};

use serde::Deserialize;

use super::{super::Packageable, RawPackageable};
use crate::context::{Context, dirs::ProjectDirs as _};

#[derive(Deserialize)]
pub(crate) struct RawCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
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

    fn cache_path(&self, context: &Context) -> PathBuf {
        let id = self.id();

        let file_name = format!("{id}.json");

        let cache_dir = context.homebrew_dirs.cache_dir();

        cache_dir.join("api/cask").join(file_name)
    }
}
