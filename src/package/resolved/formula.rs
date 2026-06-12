use std::sync::Arc;

use super::{
    super::{
        Packageable,
        raw::{Bottle, RawFormula, Versions},
    },
    ResolvedPackageable,
    ResolvedPackageableIter,
};

pub(crate) struct ResolvedFormula {
    pub(in super::super) name: String,
    pub(in super::super) versions: Versions,
    pub(in super::super) revision: u64,
    pub(in super::super) bottle: Bottle,
    dependencies: Vec<Arc<Self>>,
    pub(in super::super) keg_only: bool,
}

impl From<(RawFormula, Vec<Arc<Self>>)> for ResolvedFormula {
    fn from((raw_formula, this_dependencies): (RawFormula, Vec<Arc<Self>>)) -> Self {
        Self {
            name: raw_formula.name,
            versions: raw_formula.versions,
            revision: raw_formula.revision,
            bottle: raw_formula.bottle,
            dependencies: this_dependencies,
            keg_only: raw_formula.keg_only,
        }
    }
}

impl Packageable for ResolvedFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.versions.stable
    }
}

impl ResolvedPackageable for ResolvedFormula {}

impl ResolvedFormula {
    pub(crate) fn dependencies_mut(&mut self) -> &mut Vec<Arc<Self>> {
        &mut self.dependencies
    }
}

impl ResolvedPackageableIter for ResolvedFormula {
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
