use super::{
    super::{Packageable, prepared::PreparedCask},
    InstalledPackageable,
};

pub(crate) struct InstalledCask {
    token: String,
    version: String,
    is_requested: bool,
}

impl From<PreparedCask> for InstalledCask {
    fn from(prepared_cask: PreparedCask) -> Self {
        Self {
            token: prepared_cask.token,
            version: prepared_cask.version,
            is_requested: prepared_cask.is_requested,
        }
    }
}

impl Packageable for InstalledCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl InstalledPackageable for InstalledCask {}
