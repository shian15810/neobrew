use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::{
    super::{
        PackageExt,
        raw::formula::{Bottle, RawFormula, Versions},
    },
    ResolvedPackageExt,
};

pub(crate) struct ResolvedFormula {
    pub(in super::super) id: String,
    pub(in super::super) versions: Versions,
    pub(in super::super) revision: u64,
    pub(in super::super) bottle: Bottle,
    pub(in super::super) keg_only: bool,
    pub(in super::super) is_compatible: AtomicBool,
    pub(in super::super) is_requested: AtomicBool,

    dependencies: Vec<Arc<Self>>,
}

impl From<(RawFormula, Vec<Arc<Self>>)> for ResolvedFormula {
    fn from((raw_formula, dependencies): (RawFormula, Vec<Arc<Self>>)) -> Self {
        Self {
            id: raw_formula.name,
            versions: raw_formula.versions,
            revision: raw_formula.revision,
            bottle: raw_formula.bottle,
            keg_only: raw_formula.keg_only,
            is_compatible: AtomicBool::new(false),
            is_requested: AtomicBool::new(false),

            dependencies,
        }
    }
}

impl PackageExt for ResolvedFormula {
    fn id(&self) -> &str {
        &self.id
    }

    fn version(&self) -> &str {
        &self.versions.stable
    }
}

impl ResolvedPackageExt for ResolvedFormula {
    fn set_is_compatible(&self, is_compatible: bool) {
        self.is_compatible.store(is_compatible, Ordering::Relaxed);
    }

    fn set_is_requested(&self, is_requested: bool) {
        self.is_requested.store(is_requested, Ordering::Relaxed);
    }
}

impl ResolvedFormula {
    pub(crate) fn dependencies(&self) -> &[Arc<Self>] {
        &self.dependencies
    }

    pub(crate) fn clear_dependencies(&mut self) {
        self.dependencies.clear();

        self.dependencies.shrink_to_fit();
    }
}
