use std::sync::Arc;

pub use self::{
    cask::Cask,
    formula::{Formula, RawFormula},
};

mod cask;
mod formula;

pub enum Package {
    Formula(Arc<Formula>),
    Cask(Arc<Cask>),
}

impl Package {
    pub fn id(&self) -> &str {
        match self {
            Self::Formula(formula) => &formula.name,
            Self::Cask(cask) => &cask.token,
        }
    }
}
