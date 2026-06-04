pub(crate) mod installed;
pub(crate) mod pipelined;
pub(crate) mod prepared;
pub(crate) mod raw;
pub(crate) mod resolved;

use std::sync::Arc;

use enum_dispatch::enum_dispatch;

use self::{
    installed::InstalledPackage,
    pipelined::PipelinedPackage,
    prepared::PreparedPackage,
    raw::RawPackage,
    resolved::ResolvedPackage,
};

#[enum_dispatch]
enum Package {
    Raw(RawPackage),
    Resolved(ResolvedPackage),
    Prepared(PreparedPackage),
    Pipelined(PipelinedPackage),
    Installed(InstalledPackage),
}

#[enum_dispatch(
    Package,
    RawPackage,
    ResolvedPackage,
    PreparedPackage,
    PipelinedPackage,
    InstalledPackage
)]
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
