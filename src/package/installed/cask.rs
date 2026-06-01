use super::{
    super::{Packageable, streamed::StreamedCask},
    InstalledPackageable,
};

pub(crate) struct InstalledCask {
    token: String,
    version: String,
    is_requested: bool,
}

impl From<StreamedCask> for InstalledCask {
    fn from(streamed_cask: StreamedCask) -> Self {
        Self {
            token: streamed_cask.token,
            version: streamed_cask.version,
            is_requested: streamed_cask.is_requested,
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
