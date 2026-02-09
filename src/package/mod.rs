use std::sync::Arc;

use anyhow::{Result, anyhow};

pub use self::{cask::Cask, formula::Formula};
use crate::context::Context;

mod cask;
mod formula;

#[derive(Clone, Copy)]
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
        package: String,
        context: Arc<Context>,
        strategy: ResolutionStrategy,
    ) -> Result<Self> {
        match strategy {
            ResolutionStrategy::FormulaOnly => {
                Ok(Self::Formula(Formula::load(package, context).await?))
            },

            ResolutionStrategy::CaskOnly => Ok(Self::Cask(Cask::load(package, context).await?)),

            ResolutionStrategy::Both => {
                if let Ok(formula) = Formula::load(package.clone(), Arc::clone(&context)).await {
                    return Ok(Self::Formula(formula));
                }

                if let Ok(cask) = Cask::load(package.clone(), context).await {
                    return Ok(Self::Cask(cask));
                }

                Err(anyhow!(
                    "No available formula or cask with the name \"{package}\"."
                ))
            },
        }
    }
}

trait Loader {
    type Registry;

    fn registry(context: &Context) -> &Self::Registry;

    async fn load(package: String, context: Arc<Context>) -> Result<Arc<Self>>;
}
