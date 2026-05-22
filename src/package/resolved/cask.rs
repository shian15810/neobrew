use std::{borrow::Cow, iter, sync::Arc};

use super::{
    super::{Packageable, raw::RawCask},
    ResolvedPackageable,
    ResolvedPackageableIter,
};

pub(crate) struct ResolvedCask {
    pub(in super::super) token: String,
    version: String,
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
}

impl From<RawCask> for ResolvedCask {
    fn from(raw_cask: RawCask) -> Self {
        Self {
            token: raw_cask.token,
            version: raw_cask.version,
            url: raw_cask.url,
            sha256: raw_cask.sha256,
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

impl ResolvedPackageable for ResolvedCask {
    fn version(&self) -> Cow<'_, str> {
        let version = &self.version;

        Cow::Borrowed(version)
    }
}

impl ResolvedPackageableIter for ResolvedCask {
    fn iter(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> + use<> {
        let this = Arc::clone(self);

        iter::once(this)
    }
}
