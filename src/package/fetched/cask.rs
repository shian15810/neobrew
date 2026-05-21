use std::path::PathBuf;

use super::super::{
    Packageable,
    prepared::{PreparedCask, PreparedPackageDest},
};

pub(crate) struct FetchedCask {
    token: String,
    version: String,
    prefix_dir: PathBuf,
    caskroom_dir: PathBuf,
}

impl From<(PreparedCask, PreparedPackageDest)> for FetchedCask {
    fn from((prepared_cask, dest): (PreparedCask, PreparedPackageDest)) -> Self {
        Self {
            token: prepared_cask.token,
            version: prepared_cask.version,
            prefix_dir: dest.dir_location_greatgrandparent,
            caskroom_dir: dest.dir_location_grandparent,
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
