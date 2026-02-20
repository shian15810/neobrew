use std::sync::Arc;

use enum_dispatch::enum_dispatch;

use self::{cask::Cask, formula::Formula};

pub mod cask;
pub mod formula;

#[enum_dispatch]
pub enum Package {
    Formula(Arc<Formula>),
    Cask(Arc<Cask>),
}

#[enum_dispatch(Package)]
pub trait Packageable {
    fn id(&self) -> &str;
}

impl<Package: Packageable> Packageable for Arc<Package> {
    fn id(&self) -> &str {
        let this = &**self;

        this.id()
    }
}
