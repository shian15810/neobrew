mod cask;
mod formula;

use enum_dispatch::enum_dispatch;

use self::{cask::InstalledCask, formula::InstalledFormula};
use super::Packageable;

#[enum_dispatch]
pub(crate) enum InstalledPackage {
    Formula(InstalledFormula),
    Cask(InstalledCask),
}

#[enum_dispatch(InstalledPackage)]
trait InstalledPackageable: Packageable {}
