mod cask;
mod formula;

use std::borrow::Cow;

use enum_dispatch::enum_dispatch;

pub(crate) use self::{cask::RawCask, formula::RawFormula};
pub(super) use self::{
    cask::{Artifact, Variation},
    formula::{Bottle, BottleStable, BottleStableFile, BottleStableFileCellar, Versions},
};
use super::Packageable;

#[enum_dispatch]
pub(crate) enum RawPackage {
    Formula(RawFormula),
    Cask(RawCask),
}

#[cfg_attr(debug_assertions, expect(shadowing_supertrait_items))]
#[enum_dispatch(RawPackage)]
trait RawPackageable: Packageable {
    fn version(&self) -> Cow<'_, str>;
}
