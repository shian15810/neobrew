use std::sync::Arc;

use color_eyre::eyre::Result;
use moka::future::Cache;
use serde::Deserialize;

use crate::{context::Context, package::Loader};

#[derive(Deserialize)]
pub struct Cask {
    token: String,
    name: Vec<String>,
}

impl Loader for Cask {
    fn registry(context: &Context) -> &Cache<String, Arc<Self>> {
        context.cask_registry()
    }

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
