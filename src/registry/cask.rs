use std::sync::Arc;

use anyhow::Result;
use foyer::{Cache, CacheBuilder};

use super::Registrable;
use crate::{
    context::Context,
    package::{
        RawPackage,
        cask::{RawCask, ResolvedCask},
    },
};

pub(super) struct CaskRegistry {
    store: Cache<Arc<str>, Arc<ResolvedCask>>,

    context: Arc<Context>,
}

impl Registrable for CaskRegistry {
    type ResolvedPackage = ResolvedCask;

    const API_URL: &str = "https://formulae.brew.sh/api/cask/{}.json";

    const JSON_URL: &str = "https://formulae.brew.sh/api/cask.json";
    const JWS_JSON_URL: &str = "https://formulae.brew.sh/api/cask.jws.json";
    const TAP_MIGRATIONS_URL: &str = "https://formulae.brew.sh/api/cask_tap_migrations.json";
    const TAP_MIGRATIONS_JWS_URL: &str =
        "https://formulae.brew.sh/api/cask_tap_migrations.jws.json";

    fn new(context: Arc<Context>) -> Self {
        Self {
            store: CacheBuilder::new(usize::MAX).build(),

            context,
        }
    }

    async fn resolve(self: Arc<Self>, package: Arc<str>) -> Result<Arc<Self::ResolvedPackage>> {
        let resolved_cask = self.resolve_inner(package).await?;

        Ok(resolved_cask)
    }
}

impl CaskRegistry {
    async fn resolve_inner(self: Arc<Self>, package: Arc<str>) -> Result<Arc<ResolvedCask>> {
        let resolved_cask = self
            .store
            .get_or_fetch(&package, || {
                let this = Arc::clone(&self);

                let package = Arc::clone(&package);

                this.fetch(package)
            })
            .await?;
        let resolved_cask = resolved_cask.value();
        let resolved_cask = Arc::clone(resolved_cask);

        Ok(resolved_cask)
    }

    async fn fetch(self: Arc<Self>, package: Arc<str>) -> Result<Arc<ResolvedCask>> {
        let api_url = Self::API_URL.replace("{}", &package);

        let resp = self.context.client.get(api_url).send().await?;
        let resp = resp.error_for_status()?;

        let bytes = resp.bytes().await?;

        let raw_cask: RawCask = serde_json::from_slice(&bytes)?;

        let raw_package = RawPackage::Cask(raw_cask);

        {
            let this = Arc::as_ref(&self);

            let context = Arc::as_ref(&self.context);

            this.cache_raw_package_json(&raw_package, bytes, context)
                .await?;
        }

        #[expect(clippy::disallowed_macros)]
        let RawPackage::Cask(raw_cask) = raw_package else {
            unreachable!();
        };

        let resolved_cask = ResolvedCask::from(raw_cask);
        let resolved_cask = Arc::new(resolved_cask);

        Ok(resolved_cask)
    }
}
