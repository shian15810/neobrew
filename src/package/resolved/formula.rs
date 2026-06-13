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
    ResolvedPackageExtIter,
};

pub(crate) struct ResolvedFormula {
    pub(in super::super) name: String,
    pub(in super::super) versions: Versions,
    pub(in super::super) revision: u64,
    pub(in super::super) bottle: Bottle,
    pub(in super::super) keg_only: bool,
    pub(in super::super) is_compatible: AtomicBool,
    pub(in super::super) is_requested: AtomicBool,
    dependencies: Vec<Arc<Self>>,
}

impl From<(RawFormula, Vec<Arc<Self>>)> for ResolvedFormula {
    fn from((raw_formula, this_dependencies): (RawFormula, Vec<Arc<Self>>)) -> Self {
        Self {
            name: raw_formula.name,
            versions: raw_formula.versions,
            revision: raw_formula.revision,
            bottle: raw_formula.bottle,
            keg_only: raw_formula.keg_only,
            is_compatible: AtomicBool::new(false),
            is_requested: AtomicBool::new(false),
            dependencies: this_dependencies,
        }
    }
}

impl PackageExt for ResolvedFormula {
    fn id(&self) -> &str {
        &self.name
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
    pub(crate) fn dependencies(&self) -> &Vec<Arc<Self>> {
        &self.dependencies
    }

    pub(crate) fn dependencies_mut(&mut self) -> &mut Vec<Arc<Self>> {
        &mut self.dependencies
    }
}

impl ResolvedPackageExtIter for ResolvedFormula {
    fn iter(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> + use<> {
        let this = Arc::clone(self);

        ResolvedFormulaIter {
            stack: vec![this],
        }
    }
}

struct ResolvedFormulaIter {
    stack: Vec<Arc<ResolvedFormula>>,
}

impl Iterator for ResolvedFormulaIter {
    type Item = Arc<ResolvedFormula>;

    fn next(&mut self) -> Option<Self::Item> {
        let resolved_formula = self.stack.pop()?;

        let resolved_formula_dependencies = resolved_formula.dependencies.iter().cloned();

        self.stack.extend(resolved_formula_dependencies);

        Some(resolved_formula)
    }
}
