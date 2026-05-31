pub(crate) mod installed;
pub(crate) mod prepared;
pub(crate) mod raw;
pub(crate) mod resolved;
pub(crate) mod streamed;

use std::sync::Arc;

use enum_dispatch::enum_dispatch;

use self::{
    installed::InstalledPackage,
    prepared::PreparedPackage,
    raw::RawPackage,
    resolved::ResolvedPackage,
    streamed::StreamedPackage,
};

#[enum_dispatch]
enum Package {
    Raw(RawPackage),
    Resolved(ResolvedPackage),
    Prepared(PreparedPackage),
    Streamed(StreamedPackage),
    Installed(InstalledPackage),
}

#[enum_dispatch(
    Package,
    RawPackage,
    ResolvedPackage,
    PreparedPackage,
    StreamedPackage,
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
