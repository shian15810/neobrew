pub(crate) mod fetched;
pub(crate) mod prepared;
pub(crate) mod raw;
pub(crate) mod resolved;

use std::sync::Arc;

use enum_dispatch::enum_dispatch;

use self::{
    fetched::FetchedPackage,
    prepared::PreparedPackage,
    raw::RawPackage,
    resolved::ResolvedPackage,
};

#[enum_dispatch]
enum Package {
    Raw(RawPackage),
    Resolved(ResolvedPackage),
    Prepared(PreparedPackage),
    Fetched(FetchedPackage),
}

#[enum_dispatch(Package, RawPackage, ResolvedPackage, PreparedPackage, FetchedPackage)]
pub(crate) trait Packageable {
    fn id(&self) -> &str;

    fn version(&self) -> &str;
}

impl<Package: Packageable> Packageable for Arc<Package> {
    fn id(&self) -> &str {
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        this.id()
    }

    fn version(&self) -> &str {
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        this.version()
    }
}
