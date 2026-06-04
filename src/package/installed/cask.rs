use super::{
    super::{Packageable, pipelined::PipelinedCask},
    InstalledPackageable,
};

pub(crate) struct InstalledCask {
    token: String,
    version: String,
    is_requested: bool,
}

impl From<PipelinedCask> for InstalledCask {
    fn from(pipelined_cask: PipelinedCask) -> Self {
        Self {
            token: pipelined_cask.token,
            version: pipelined_cask.version,
            is_requested: pipelined_cask.is_requested,
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
