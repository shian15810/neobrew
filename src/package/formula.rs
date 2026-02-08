use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Result, anyhow};
use async_recursion::async_recursion;
use futures::future;
use serde::Deserialize;

use super::Loader;
use crate::context::{Context, FormulaRegistry};

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
    #[async_recursion]
    async fn fetch_with_stack(
        package: String,
        context: Arc<Context>,
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

        let dependencies = raw_formula.dependencies.iter().map(|dependency| {
            Self::load_with_stack(dependency, Arc::clone(&context), stack.clone())
        });
        let dependencies = future::try_join_all(dependencies).await?;

        let formula = raw_formula.into_formula(dependencies);

        Ok(Arc::new(formula))
    }

    async fn load_with_stack(
        package: &str,
        context: Arc<Context>,
        mut stack: Vec<String>,
    ) -> Result<Arc<Self>> {
        let package = package.to_owned();

        if stack.contains(&package) {
            stack.push(package);

            return Err(anyhow!(
                "Circular dependency detected: {}",
                stack.join(" -> ")
            ));
        }

        stack.push(package.clone());

        let formula = Self::registry(&context)
            .get_or_fetch(&package, || {
                Self::fetch_with_stack(package.clone(), Arc::clone(&context), stack)
            })
            .await
            .map(|entry| Arc::clone(entry.value()))?;

        Ok(formula)
    }
}

impl Loader for Formula {
    type Registry = FormulaRegistry;

    fn registry(context: &Context) -> &Self::Registry {
        context.formula_registry()
    }

    async fn load(package: &str, context: Arc<Context>) -> Result<Arc<Self>> {
        Self::load_with_stack(package, context, Vec::new()).await
    }
}
