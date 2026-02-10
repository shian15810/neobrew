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
