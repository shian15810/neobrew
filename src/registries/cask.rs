use std::sync::Arc;

use anyhow::Result;
use foyer::{Cache, CacheBuilder};

use super::Registry;
use crate::{context::Context, package::cask::Cask};

pub struct CaskRegistry {
    store: Cache<String, Arc<Cask>>,

    context: Arc<Context>,
}

impl CaskRegistry {
    async fn resolve_inner(self: Arc<Self>, package: String) -> Result<Arc<Cask>> {
        let cask = self
            .store
            .get_or_fetch(&package, || {
                let this = Arc::clone(&self);

                this.fetch(package.clone())
            })
            .await
            .map(|entry| Arc::clone(entry.value()))?;

        Ok(cask)
    }

    async fn fetch(self: Arc<Self>, package: String) -> Result<Arc<Cask>> {
        let url = format!("https://formulae.brew.sh/api/cask/{package}.json");

        let cask: Cask = self
            .context
            .http_client()
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(Arc::new(cask))
    }
}

impl Registry for CaskRegistry {
    type Package = Cask;

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

    async fn resolve(self: Arc<Self>, package: String) -> Result<Arc<Self::Package>> {
        self.resolve_inner(package).await
    }
}
