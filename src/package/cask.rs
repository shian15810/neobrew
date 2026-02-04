use color_eyre::eyre::Result;
use serde::Deserialize;

use crate::{context::Context, package::Loader};

#[derive(Deserialize)]
pub struct Cask {
    token: String,
    name: Vec<String>,
}

impl Loader for Cask {
    async fn load(package: &str, context: &Context) -> Result<Self> {
        let cask_url = format!("https://formulae.brew.sh/api/cask/{package}.json");

        let cask = context
            .client()
            .get(&cask_url)
            .send()
            .await?
            .error_for_status()?
            .json::<Self>()
            .await?;

        Ok(cask)
    }
}
