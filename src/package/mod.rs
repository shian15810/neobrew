use color_eyre::eyre::{Result, eyre};

use crate::{
    context::Context,
    package::{cask::Cask, formula::Formula},
};

mod cask;
mod formula;

pub enum Package {
    Formula(Formula),
    Cask(Cask),
}

impl Package {
    pub async fn resolve(
        package: &str,
        context: &Context,
        strategy: &ResolutionStrategy,
    ) -> Result<Self> {
        match strategy {
            ResolutionStrategy::FormulaOnly => {
                Ok(Self::Formula(Formula::load(package, context).await?))
            },
            ResolutionStrategy::CaskOnly => Ok(Self::Cask(Cask::load(package, context).await?)),
            ResolutionStrategy::Both => {
                if let Ok(formula) = Formula::load(package, context).await {
                    return Ok(Self::Formula(formula));
                }

                if let Ok(cask) = Cask::load(package, context).await {
                    return Ok(Self::Cask(cask));
                }

                Err(eyre!(
                    "No available formula or cask with the name \"{package}\"."
                ))
            },
        }
    }
}

pub enum ResolutionStrategy {
    FormulaOnly,
    CaskOnly,
    Both,
}

trait Loader: Sized {
    async fn load(package: &str, context: &Context) -> Result<Self>;
}
