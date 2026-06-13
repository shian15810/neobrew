use std::{
    collections::HashMap,
    iter,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use super::{
    super::{
        PackageExt,
        raw::cask::{Artifact, RawCask, Variation},
    },
    ResolvedPackageExt,
    ResolvedPackageExtIter,
};

pub(crate) struct ResolvedCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
    pub(in super::super) artifacts: Vec<Artifact>,
    pub(in super::super) variations: HashMap<String, Variation>,
    pub(in super::super) is_compatible: AtomicBool,
    pub(in super::super) is_requested: AtomicBool,
}

impl From<RawCask> for ResolvedCask {
    fn from(raw_cask: RawCask) -> Self {
        Self {
            token: raw_cask.token,
            version: raw_cask.version,
            url: raw_cask.url,
            sha256: raw_cask.sha256,
            artifacts: raw_cask.artifacts,
            variations: raw_cask.variations,
            is_compatible: AtomicBool::new(false),
            is_requested: AtomicBool::new(false),
        }
    }
}

impl PackageExt for ResolvedCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl ResolvedPackageExt for ResolvedCask {
    fn set_is_compatible(&self, is_compatible: bool) {
        self.is_compatible.store(is_compatible, Ordering::Relaxed);
    }

    fn set_is_requested(&self, is_requested: bool) {
        self.is_requested.store(is_requested, Ordering::Relaxed);
    }
}

impl ResolvedPackageExtIter for ResolvedCask {
    fn iter(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> + use<> {
        let this = Arc::clone(self);

        iter::once(this)
    }
}
