mod cask;
mod formula;

use std::{borrow::Cow, path::PathBuf};

use enum_dispatch::enum_dispatch;

pub(super) use self::formula::{
    Bottle,
    BottleStable,
    BottleStableFile,
    BottleStableFileCellar,
    Versions,
};
pub(crate) use self::{cask::RawCask, formula::RawFormula};
use super::Packageable;
use crate::context::Context;

#[enum_dispatch]
pub(crate) enum RawPackage {
    Formula(RawFormula),
    Cask(RawCask),
}

#[cfg_attr(debug_assertions, expect(shadowing_supertrait_items))]
#[enum_dispatch(RawPackage)]
pub(crate) trait RawPackageable: Packageable {
    fn version(&self) -> Cow<'_, str>;

    fn cache_path(&self, context: &Context) -> PathBuf;
}
