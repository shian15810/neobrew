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

impl RawFormula {
    pub fn into_formula(self, dependencies: Vec<Arc<Formula>>) -> Formula {
        Formula {
            name: self.name,
            versions: self.versions,
            revision: self.revision,
            bottle: self.bottle,
            dependencies,
        }
    }
}

pub struct Formula {
    name: String,
    versions: Versions,
    revision: u64,
    bottle: Bottle,
    dependencies: Vec<Arc<Self>>,
}

impl Formula {
    pub fn iter(self: &Arc<Self>) -> FormulaIter {
        FormulaIter {
            stack: vec![Arc::clone(self)],
        }
    }
}

impl Packageable for Formula {
    fn id(&self) -> &str {
        &self.name
    }
}

pub struct FormulaIter {
    stack: Vec<Arc<Formula>>,
}

impl Iterator for FormulaIter {
    type Item = Arc<Formula>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.stack.pop()?;

        let dependencies = current.dependencies.iter().cloned();

        self.stack.extend(dependencies);

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
