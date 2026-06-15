pub(crate) mod cask;
pub(crate) mod formula;

use std::{collections::HashSet, sync::Arc};

use enum_dispatch::enum_dispatch;

use self::{cask::ResolvedCask, formula::ResolvedFormula};
use super::PackageExt;

#[enum_dispatch]
pub(crate) enum ResolvedPackage {
    Formula(Arc<ResolvedFormula>),
    Cask(Arc<ResolvedCask>),
}

#[enum_dispatch(ResolvedPackage)]
pub(crate) trait ResolvedPackageExt: PackageExt {
    fn set_is_compatible(&self, is_compatible: bool);

    fn set_is_requested(&self, is_requested: bool);
}

impl<ResolvedPackage: ResolvedPackageExt> ResolvedPackageExt for Arc<ResolvedPackage> {
    fn set_is_compatible(&self, is_compatible: bool) {
        ResolvedPackage::set_is_compatible(self, is_compatible);
    }

    fn set_is_requested(&self, is_requested: bool) {
        ResolvedPackage::set_is_requested(self, is_requested);
    }
}

impl IntoIterator for ResolvedPackage {
    type Item = Self;
    type IntoIter = ResolvedPackageIter;

    fn into_iter(self) -> Self::IntoIter {
        ResolvedPackageIter {
            stack: vec![self],

            visited: HashSet::new(),
        }
    }
}

pub(crate) struct ResolvedPackageIter {
    stack: Vec<ResolvedPackage>,

    visited: HashSet<String>,
}

impl Iterator for ResolvedPackageIter {
    type Item = ResolvedPackage;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(resolved_package) = self.stack.pop() {
            let id = resolved_package.id();
            let id = id.to_owned();

            if !self.visited.insert(id) {
                continue;
            }

            match &resolved_package {
                ResolvedPackage::Formula(resolved_formula) => {
                    let dependencies = resolved_formula
                        .dependencies()
                        .iter()
                        .cloned()
                        .map(ResolvedPackage::Formula);

                    self.stack.extend(dependencies);
                },
                ResolvedPackage::Cask(resolved_cask) => {
                    let dependencies = resolved_cask
                        .dependencies()
                        .iter()
                        .cloned()
                        .map(ResolvedPackage::Cask);

                    self.stack.extend(dependencies);

                    let formula_dependencies = resolved_cask
                        .formula_dependencies()
                        .iter()
                        .cloned()
                        .map(ResolvedPackage::Formula);

                    self.stack.extend(formula_dependencies);
                },
            }

            return Some(resolved_package);
        }

        None
    }
}
