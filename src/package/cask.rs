use std::sync::Arc;

use color_eyre::eyre::{Result, eyre};
use serde::Deserialize;

use crate::{context::Context, package::Loader};

#[derive(Deserialize)]
pub struct Cask {
    token: String,
    name: Vec<String>,
}

impl Cask {
    async fn fetch(name: String, context: &Context) -> Result<Arc<Self>> {
        let cask_url = format!("https://formulae.brew.sh/api/cask/{name}.json");

        let cask: Self = context
            .client()
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
        let name = package.to_string();

        context
            .cask_registry()
            .try_get_with(name.clone(), Self::fetch(name, context))
            .await
            .map_err(|e| eyre!(e))
    }
}
