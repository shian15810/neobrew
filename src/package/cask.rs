use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::Loader;
use crate::context::Context;

#[derive(Serialize, Deserialize)]
pub struct Cask {
    token: String,
    name: Vec<String>,
    url: String,
    version: String,
    sha256: String,
}

impl Cask {
    async fn fetch(package: String, context: Arc<Context>) -> Result<Arc<Self>> {
        let cask_url = format!("https://formulae.brew.sh/api/cask/{package}.json");

        let cask: Self = context
            .http_client()
            .get(&cask_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(Arc::new(cask))
    }
}

impl Loader for Cask {
    async fn load(package: &str, context: Arc<Context>) -> Result<Arc<Self>> {
        let cask = context
            .cask_registry()
            .await?
            .get_or_fetch(package, || {
                Self::fetch(package.to_owned(), Arc::clone(&context))
            })
            .await
            .map(|entry| Arc::clone(entry.value()))?;

        Ok(cask)
    }
}
