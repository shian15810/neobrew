use super::{
    super::{Packageable, resolved::ResolvedCask},
    PreparedPackageable,
};

pub(crate) struct PreparedCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    url: String,
    sha256: String,
}

impl From<ResolvedCask> for PreparedCask {
    fn from(resolved_cask: ResolvedCask) -> Self {
        #[cfg(debug_assertions)]
        #[cfg_attr(
            debug_assertions,
            expect(resolving_to_items_shadowing_supertrait_items)
        )]
        let version = {
            use super::super::resolved::ResolvedPackageable as _;

            resolved_cask.version()
        };

        #[cfg(not(debug_assertions))]
        let version = {
            use super::super::resolved::ResolvedPackageable;

            ResolvedPackageable::version(&resolved_cask)
        };

        let version = version.into_owned();

        Self {
            token: resolved_cask.token,
            version,
            url: resolved_cask.url,
            sha256: resolved_cask.sha256,
        }
    }
}

impl Packageable for PreparedCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl PreparedPackageable for PreparedCask {
    fn cache_url(&self) -> &str {
        &self.url
    }

    fn expected_sha256(&self) -> &str {
        &self.sha256
    }
}

impl PreparedCask {
    pub(crate) fn url(&self) -> &str {
        &self.url
    }
}
