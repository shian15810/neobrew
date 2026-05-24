use super::super::{Packageable, prepared::PreparedCask};

pub(crate) struct FetchedCask {
    token: String,
    version: String,
}

impl From<PreparedCask> for FetchedCask {
    fn from(prepared_cask: PreparedCask) -> Self {
        Self {
            token: prepared_cask.token,
            version: prepared_cask.version,
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
