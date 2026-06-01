use std::{collections::HashMap, iter, sync::Arc};

use super::{
    super::{
        Packageable,
        raw::{Artifact, DependsOn, RawCask, Variation},
    },
    ResolvedPackageable,
    ResolvedPackageableIter,
};

pub(crate) struct ResolvedCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
    pub(in super::super) artifacts: Vec<Artifact>,
    depends_on: DependsOn,
    pub(in super::super) variations: HashMap<String, Variation>,
}

impl From<RawCask> for ResolvedCask {
    fn from(raw_cask: RawCask) -> Self {
        Self {
            token: raw_cask.token,
            version: raw_cask.version,
            url: raw_cask.url,
            sha256: raw_cask.sha256,
            artifacts: raw_cask.artifacts,
            depends_on: raw_cask.depends_on,
            variations: raw_cask.variations,
        }
    }
}

impl Packageable for ResolvedCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl ResolvedPackageable for ResolvedCask {}

impl ResolvedCask {
    pub(crate) fn depends_on(&self) -> &DependsOn {
        &self.depends_on
    }
}

impl ResolvedPackageableIter for ResolvedCask {
    fn iter(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> + use<> {
        let this = Arc::clone(self);

        iter::once(this)
    }
}
