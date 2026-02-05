use std::sync::Arc;

use color_eyre::eyre::{Result, eyre};

pub use self::{cask::Cask, formula::Formula};
use crate::context::Context;

mod cask;
mod formula;

pub enum ResolutionStrategy {
    FormulaOnly,
    CaskOnly,
    Both,
}

pub enum Package {
    Formula(Arc<Formula>),
    Cask(Arc<Cask>),
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

trait Loader {
    async fn load(package: &str, context: &Context) -> Result<Arc<Self>>;
}
