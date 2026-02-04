use color_eyre::eyre::Result;
use serde::Deserialize;

use crate::{context::Context, package::Loader};

#[derive(Deserialize)]
pub struct Formula {
    name: String,
    dependencies: Vec<String>,
}

impl Loader for Formula {
    async fn load(package: &str, context: &Context) -> Result<Self> {
        let formula_url = format!("https://formulae.brew.sh/api/formula/{package}.json");

        let formula = context
            .client()
            .get(&formula_url)
            .send()
            .await?
            .error_for_status()?
            .json::<Self>()
            .await?;

        Ok(formula)
    }
}
