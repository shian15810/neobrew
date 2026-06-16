pub(crate) mod cask;
pub(crate) mod formula;

use enum_dispatch::enum_dispatch;

use self::{cask::RawCask, formula::RawFormula};
use super::PackageExt;

#[enum_dispatch]
pub(crate) enum RawPackage {
    Formula(RawFormula),
    Cask(RawCask),
}

#[enum_dispatch(RawPackage)]
trait RawPackageExt: PackageExt {}
