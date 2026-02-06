use std::{collections::BTreeMap, sync::Arc};

use color_eyre::eyre::{Result, eyre};
use futures::future;
use serde::Deserialize;

use super::Loader;
use crate::context::Context;

#[derive(Deserialize)]
struct RawFormula {
    name: String,
    versions: Versions,
    revision: u64,
    bottle: Bottle,
    dependencies: Vec<String>,
}

impl RawFormula {
    fn into_formula(self, dependencies: Vec<Arc<Formula>>) -> Formula {
        Formula {
            name: self.name,
            versions: self.versions,
            revision: self.revision,
            bottle: self.bottle,
            dependencies,
        }
    }
}

pub struct Formula {
    name: String,
    versions: Versions,
    revision: u64,
    bottle: Bottle,
    dependencies: Vec<Arc<Self>>,
}

#[derive(Deserialize)]
struct Versions {
    stable: String,
}

#[derive(Deserialize)]
struct Bottle {
    stable: BottleStable,
}

#[derive(Deserialize)]
struct BottleStable {
    rebuild: u64,
    files: BTreeMap<String, BottleStableFile>,
}

#[derive(Deserialize)]
struct BottleStableFile {
    url: String,
    sha256: String,
}

impl Formula {
    async fn fetch_with_stack(
        package: &str,
        context: &Context,
        stack: Vec<String>,
    ) -> Result<Arc<Self>> {
        let formula_url = format!("https://formulae.brew.sh/api/formula/{package}.json");

        let raw_formula: RawFormula = context
            .http_client()
            .get(&formula_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let dependencies = raw_formula
            .dependencies
            .iter()
            .map(|dependency| Self::load_with_stack(dependency, context, stack.clone()));
        let dependencies = future::try_join_all(dependencies).await?;

        let formula = raw_formula.into_formula(dependencies);

        Ok(Arc::new(formula))
    }

    async fn load_with_stack(
        package: &str,
        context: &Context,
        mut stack: Vec<String>,
    ) -> Result<Arc<Self>> {
        let package = package.to_string();

        if stack.contains(&package) {
            stack.push(package);

            return Err(eyre!(
                "Circular dependency detected: {}",
                stack.join(" -> ")
            ));
        }

        stack.push(package.clone());

        context
            .formula_registry()
            .try_get_with(
                package.clone(),
                Self::fetch_with_stack(&package, context, stack),
            )
            .await
            .map_err(|e| eyre!(e))
    }
}

impl Loader for Formula {
    async fn load(package: &str, context: &Context) -> Result<Arc<Self>> {
        Self::load_with_stack(package, context, Vec::new()).await
    }
}
