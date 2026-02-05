use std::sync::Arc;

use color_eyre::eyre::Result;
use futures::future;
use moka::future::Cache;
use serde::Deserialize;

use super::Loader;
use crate::context::Context;

pub struct Formula {
    name: String,
    dependencies: Vec<Arc<Self>>,
}

#[derive(Deserialize)]
struct RawFormula {
    name: String,
    dependencies: Vec<String>,
}

impl Loader for Formula {
    fn registry(context: &Context) -> &Cache<String, Arc<Self>> {
        context.formula_registry()
    }

    async fn fetch(package: &str, context: &Context) -> Result<Arc<Self>> {
        let formula_url = format!("https://formulae.brew.sh/api/formula/{package}.json");

        let raw_formula: RawFormula = context
            .http_client()
            .get(&formula_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let RawFormula { name, dependencies } = raw_formula;

        let dependencies = dependencies
            .iter()
            .map(|dependency| Self::load(dependency, context));
        let dependencies = future::try_join_all(dependencies).await?;

        let formula = Self { name, dependencies };

        Ok(Arc::new(formula))
    }
}
