use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;

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
    pub name: String,
    versions: Versions,
    revision: u64,
    bottle: Bottle,
    dependencies: Vec<Arc<Self>>,
}

impl Formula {}

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
