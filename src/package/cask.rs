use std::sync::Arc;

use color_eyre::eyre::{Result, eyre};
use serde::Deserialize;

use super::Loader;
use crate::context::Context;

#[derive(Deserialize)]
pub struct Cask {
    token: String,
    name: Vec<String>,
    url: String,
    version: String,
    sha256: String,
}

impl Cask {
    async fn fetch(package: &str, context: &Context) -> Result<Arc<Self>> {
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
    async fn load(package: &str, context: &Context) -> Result<Arc<Self>> {
        context
            .cask_registry()
            .try_get_with(package.to_string(), Self::fetch(package, context))
            .await
            .map_err(|e| eyre!(e))
    }
}
