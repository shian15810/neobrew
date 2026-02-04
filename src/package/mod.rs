use anyhow::Result;
use futures::future;
use serde::Deserialize;

use crate::context::Context;

mod cask;
mod formula;

pub enum Package {
    Formula(Formula),
    Cask(Cask),
}

impl Package {
    pub async fn resolve(package: &str, context: &Context) -> Result<Self> {
        let (formula_res, cask_res) = future::join(
            Formula::load(package, context),
            Cask::load(package, context),
        )
        .await;

        match (formula_res, cask_res) {
            (Ok(formula), _) => Ok(Self::Formula(formula)),
            (Err(_), Ok(cask)) => Ok(Self::Cask(cask)),
            (Err(formula_err), Err(cask_err)) => Err(anyhow::anyhow!("")),
        }
    }
}

trait Loader: Sized {
    async fn load(package: &str, context: &Context) -> Result<Self>;
}

#[derive(Deserialize)]
struct Formula {
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

#[derive(Deserialize)]
struct Cask {
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
