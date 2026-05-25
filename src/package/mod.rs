pub(crate) mod fetched;
pub(crate) mod prepared;
pub(crate) mod raw;
pub(crate) mod resolved;

use std::sync::Arc;

use enum_dispatch::enum_dispatch;

use self::{
    fetched::{FetchedCask, FetchedFormula, FetchedPackage},
    prepared::{PreparedCask, PreparedFormula, PreparedPackage},
    raw::{RawCask, RawFormula, RawPackage},
    resolved::{ResolvedCask, ResolvedFormula, ResolvedPackage},
};

#[enum_dispatch]
enum Package {
    Raw(RawPackage),
    Resolved(ResolvedPackage),
    Prepared(PreparedPackage),
    Fetched(FetchedPackage),
}

#[enum_dispatch(Package, Formula, Cask, RawPackage, ResolvedPackage, PreparedPackage, FetchedPackage)]
pub(crate) trait Packageable {
    fn id(&self) -> &str;

    fn version(&self) -> &str;
}

impl<Package: Packageable> Packageable for Arc<Package> {
    fn id(&self) -> &str {
        Package::id(self)
    }

    fn version(&self) -> &str {
        Package::version(self)
    }
}

#[enum_dispatch]
enum Formula {
    Raw(RawFormula),
    Resolved(ResolvedFormula),
    Prepared(PreparedFormula),
    Fetched(FetchedFormula),
}

#[enum_dispatch(Formula, RawFormula, ResolvedFormula, PreparedFormula, FetchedFormula)]
pub(crate) trait Formulable: Packageable {
    fn keg_only(&self) -> bool;
}

impl<Formula: Formulable> Formulable for Arc<Formula> {
    fn keg_only(&self) -> bool {
        Formula::keg_only(self)
    }
}

#[enum_dispatch]
enum Cask {
    Raw(RawCask),
    Resolved(ResolvedCask),
    Prepared(PreparedCask),
    Fetched(FetchedCask),
}

#[enum_dispatch(Cask, RawCask, ResolvedCask, PreparedCask, FetchedCask)]
pub(crate) trait Caskable: Packageable {}

impl<Cask: Caskable> Caskable for Arc<Cask> {}
