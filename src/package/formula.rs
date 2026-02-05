use std::sync::Arc;

use color_eyre::eyre::{Result, eyre};
use futures::future;
use serde::Deserialize;

use crate::{context::Context, package::Loader};

pub struct Formula {
    name: String,
    dependencies: Vec<Arc<Self>>,
}

#[derive(Deserialize)]
struct RawFormula {
    name: String,
    dependencies: Vec<String>,
}

impl Formula {
    async fn fetch(name: String, context: &Context) -> Result<Arc<Self>> {
        let formula_url = format!("https://formulae.brew.sh/api/formula/{name}.json");

        let raw_formula: RawFormula = context
            .client()
            .get(&formula_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let dependencies = raw_formula
            .dependencies
            .iter()
            .map(|dep| Self::load(dep, context));
        let dependencies = future::try_join_all(dependencies).await?;

        let formula = Self { name, dependencies };

        Ok(Arc::new(formula))
    }
}

impl Loader for Formula {
    async fn load(package: &str, context: &Context) -> Result<Arc<Self>> {
        let name = package.to_string();

        context
            .formula_registry()
            .try_get_with(name.clone(), Self::fetch(name, context))
            .await
            .map_err(|e| eyre!(e))
    }
}
