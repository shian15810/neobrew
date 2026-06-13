use std::{path::PathBuf, sync::Arc};

use foyer::{Cache, CacheBuilder};

use super::{
    RegistryExt,
    RegistryJsonExt,
    compatibility::{CaskCompatibility as _, Compatibility},
};
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    package::{
        PackageExt as _,
        raw::cask::RawCask,
        resolved::{ResolvedPackageExt as _, cask::ResolvedCask},
    },
};

pub(super) struct CaskRegistry {
    store: Cache<Arc<str>, Arc<ResolvedCask>>,

    compatibility: Arc<Compatibility>,

    context: Arc<Context>,
}

impl RegistryExt for CaskRegistry {
    type ResolvedPackage = ResolvedCask;

    const API_URL: &str = "https://formulae.brew.sh/api/cask/{}.json";

    const JSON_URL: &str = "https://formulae.brew.sh/api/cask.json";
    const JWS_JSON_URL: &str = "https://formulae.brew.sh/api/cask.jws.json";

    const TAP_MIGRATIONS_URL: &str = "https://formulae.brew.sh/api/cask_tap_migrations.json";
    const TAP_MIGRATIONS_JWS_URL: &str =
        "https://formulae.brew.sh/api/cask_tap_migrations.jws.json";

    fn new(compatibility: Arc<Compatibility>, context: Arc<Context>) -> Self {
        Self {
            store: CacheBuilder::new(usize::MAX).build(),

            compatibility,

            context,
        }
    }

    async fn resolve(self: Arc<Self>, package: Arc<str>) -> anyhow::Result<Arc<ResolvedCask>> {
        let resolved_cask = self.resolve_inner(package).await?;

        Ok(resolved_cask)
    }
}

impl CaskRegistry {
    async fn resolve_inner(
        self: Arc<Self>,
        package: Arc<str>,
    ) -> anyhow::Result<Arc<ResolvedCask>> {
        let entry = self
            .store
            .get_or_fetch(&package, || {
                let this = Arc::clone(&self);

                this.fetch(Arc::clone(&package))
            })
            .await?;

        let resolved_cask = Arc::clone(entry.value());

        Ok(resolved_cask)
    }

    async fn fetch(self: Arc<Self>, package: Arc<str>) -> anyhow::Result<Arc<ResolvedCask>> {
        let api_url = Self::API_URL.replace("{}", &package);

        let resp = self.context.client.get(api_url).send().await?;
        let resp = resp.error_for_status()?;

        let bytes = resp.bytes().await?;

        let raw_cask: RawCask = serde_json::from_slice(&bytes)?;

        self.save_json(raw_cask.id(), bytes).await?;

        let is_compatible = self.compatibility.is_cask_compatible(&raw_cask);

        let resolved_cask = ResolvedCask::from(raw_cask);
        let resolved_cask = Arc::new(resolved_cask);

        resolved_cask.set_is_compatible(is_compatible);

        Ok(resolved_cask)
    }
}

impl RegistryJsonExt for CaskRegistry {
    fn json_path(&self, id: &str) -> PathBuf {
        let file_name = format!("{id}.json");

        let cache_dir_path = self.context.homebrew_dirs.cache_dir();

        cache_dir_path.join("api/cask").join(file_name)
    }
}
