use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;

use super::Packageable;

#[derive(Deserialize)]
pub struct RawFormula {
    name: String,
    versions: Versions,
    revision: u64,
    bottle: Bottle,
    pub dependencies: Vec<String>,
}

impl Packageable for RawFormula {
    fn id(&self) -> &str {
        &self.name
    }
}

pub struct ResolvedFormula {
    name: String,
    versions: Versions,
    revision: u64,
    bottle: Bottle,
    dependencies: Vec<Arc<Self>>,
}

impl From<(RawFormula, Vec<Arc<Self>>)> for ResolvedFormula {
    fn from((raw, dependencies): (RawFormula, Vec<Arc<Self>>)) -> Self {
        Self {
            name: raw.name,
            versions: raw.versions,
            revision: raw.revision,
            bottle: raw.bottle,
            dependencies,
        }
    }
}

impl Packageable for ResolvedFormula {
    fn id(&self) -> &str {
        &self.name
    }
}

impl ResolvedFormula {
    pub fn iter(self: &Arc<Self>) -> ResolvedFormulaIter {
        ResolvedFormulaIter {
            stack: vec![Arc::clone(self)],
        }
    }
}

pub struct ResolvedFormulaIter {
    stack: Vec<Arc<ResolvedFormula>>,
}

impl Iterator for ResolvedFormulaIter {
    type Item = Arc<ResolvedFormula>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.stack.pop()?;

        let children = current.dependencies.iter().cloned();

        self.stack.extend(children);

        Some(current)
    }
}

#[derive(Deserialize)]
struct Versions {
    stable: String,
}

#[derive(Deserialize)]
struct Bottle {
    stable: BottleStable,
}

#[derive(Deserialize)]
struct BottleStable {
    rebuild: u64,
    files: HashMap<String, BottleStableFile>,
}

#[derive(Deserialize)]
struct BottleStableFile {
    url: String,
    sha256: String,
}
