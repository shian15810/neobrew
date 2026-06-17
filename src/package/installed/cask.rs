use super::{
    super::{PackageExt, prepared::cask::PreparedCask},
    InstalledPackageExt,
};

pub(crate) struct InstalledCask {
    id: String,
    version: String,
    is_requested: bool,
}

impl From<PreparedCask> for InstalledCask {
    fn from(prepared_cask: PreparedCask) -> Self {
        Self {
            id: prepared_cask.id,
            version: prepared_cask.version,
            is_requested: prepared_cask.is_requested,
        }
    }
}

impl PackageExt for InstalledCask {
    fn id(&self) -> &str {
        &self.id
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl InstalledPackageExt for InstalledCask {}
