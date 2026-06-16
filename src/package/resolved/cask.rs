use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use super::{
    super::{
        PackageExt,
        raw::cask::{Artifact, RawCask, Variation},
    },
    ResolvedPackageExt,
    formula::ResolvedFormula,
};

pub(crate) struct ResolvedCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
    pub(in super::super) artifacts: Vec<Artifact>,
    pub(in super::super) variations: HashMap<String, Variation>,
    pub(in super::super) is_compatible: AtomicBool,
    pub(in super::super) is_requested: AtomicBool,

    dependencies: Vec<Arc<Self>>,
    formula_dependencies: Vec<Arc<ResolvedFormula>>,
}

impl From<(RawCask, Vec<Arc<Self>>, Vec<Arc<ResolvedFormula>>)> for ResolvedCask {
    fn from(
        (raw_cask, dependencies, formula_dependencies): (
            RawCask,
            Vec<Arc<Self>>,
            Vec<Arc<ResolvedFormula>>,
        ),
    ) -> Self {
        Self {
            token: raw_cask.token,
            version: raw_cask.version,
            url: raw_cask.url,
            sha256: raw_cask.sha256,
            artifacts: raw_cask.artifacts,
            variations: raw_cask.variations,
            is_compatible: AtomicBool::new(false),
            is_requested: AtomicBool::new(false),

            dependencies,
            formula_dependencies,
        }
    }
}

impl PackageExt for ResolvedCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl ResolvedPackageExt for ResolvedCask {
    fn set_is_compatible(&self, is_compatible: bool) {
        self.is_compatible.store(is_compatible, Ordering::Relaxed);
    }

    fn set_is_requested(&self, is_requested: bool) {
        self.is_requested.store(is_requested, Ordering::Relaxed);
    }
}

impl ResolvedCask {
    pub(crate) fn dependencies(&self) -> &[Arc<Self>] {
        &self.dependencies
    }

    pub(crate) fn formula_dependencies(&self) -> &[Arc<ResolvedFormula>] {
        &self.formula_dependencies
    }

    pub(crate) fn clear_dependencies(&mut self) {
        self.dependencies.clear();
    }

    pub(crate) fn clear_formula_dependencies(&mut self) {
        self.formula_dependencies.clear();
    }
}
