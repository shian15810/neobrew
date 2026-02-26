use std::sync::Arc;

use anyhow::Result;
use etcetera::AppStrategy;
use foyer::{Cache, CacheBuilder};
use serde_json::Value;
use tokio::fs;

use super::Registrable;
use crate::{
    context::Context,
    package::{
        Packageable,
        cask::{RawCask, ResolvedCask},
    },
};

pub struct CaskRegistry {
    store: Cache<String, Arc<ResolvedCask>>,

    context: Arc<Context>,
}

impl Registrable for CaskRegistry {
    type ResolvedPackage = ResolvedCask;

    const API_URL: &str = "https://formulae.brew.sh/api/cask/{}.json";

    const JSON_URL: &str = "https://formulae.brew.sh/api/cask.json";
    const JWS_JSON_URL: &str = "https://formulae.brew.sh/api/cask.jws.json";
    const TAP_MIGRATIONS_URL: &str = "https://formulae.brew.sh/api/cask_tap_migrations.json";
    const TAP_MIGRATIONS_JWS_URL: &str = "https://formulae.brew.sh/api/cask_tap_migrations.jws.json";

    fn new(context: Arc<Context>) -> Self {
        Self {
            store: CacheBuilder::new(usize::MAX).build(),

            context,
        }
    }

    async fn resolve(self: Arc<Self>, package: String) -> Result<Arc<Self::ResolvedPackage>> {
        let resolved_cask = self.resolve_inner(package).await?;

        Ok(resolved_cask)
    }
}

impl CaskRegistry {
    async fn resolve_inner(self: Arc<Self>, package: String) -> Result<Arc<ResolvedCask>> {
        let resolved_cask = self
            .store
            .get_or_fetch(&package, || {
                let this = Arc::clone(&self);

                this.fetch(package.clone())
            })
            .await
            .map(|entry| Arc::clone(entry.value()))?;

        Ok(resolved_cask)
    }

    async fn fetch(self: Arc<Self>, package: String) -> Result<Arc<ResolvedCask>> {
        let url = Self::API_URL.replace("{}", &package);

        let resp = self
            .context
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?;

        let value: Value = resp.json().await?;

        let bytes = serde_json::to_vec(&value)?;

        let raw_cask: RawCask = serde_json::from_value(value)?;

        let resolved_cask = ResolvedCask::from(raw_cask);
        let resolved_cask = Arc::new(resolved_cask);

        let dir = self
            .context
            .project_dirs()?
            .cache_dir()
            .join("api")
            .join("cask");

        fs::create_dir_all(&dir).await?;

        let file = dir.join(format!("{}.json", resolved_cask.id()));

        fs::write(file, bytes).await?;

        Ok(resolved_cask)
    }
}
