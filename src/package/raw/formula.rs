use std::{collections::HashMap, path::PathBuf, str::FromStr};

use serde::Deserialize;
use serde_with::DeserializeFromStr;

use super::{super::Packageable, RawPackageable};

#[derive(Deserialize)]
pub(crate) struct RawFormula {
    pub(in super::super) name: String,
    pub(in super::super) versions: Versions,
    pub(in super::super) revision: u64,
    pub(in super::super) bottle: Bottle,
    dependencies: Vec<String>,
    pub(in super::super) keg_only: bool,
}

impl RawFormula {
    pub(crate) fn dependencies(&self) -> &[String] {
        &self.dependencies
    }
}

impl Packageable for RawFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.versions.stable
    }
}

impl RawPackageable for RawFormula {}

#[derive(Deserialize)]
pub(in super::super) struct Versions {
    pub(in super::super) stable: String,
}

#[derive(Deserialize)]
pub(in super::super) struct Bottle {
    pub(in super::super) stable: BottleStable,
}

#[derive(Deserialize)]
pub(in super::super) struct BottleStable {
    pub(in super::super) rebuild: u64,
    pub(in super::super) files: HashMap<String, BottleStableFile>,
}

#[derive(Deserialize)]
pub(in super::super) struct BottleStableFile {
    pub(in super::super) cellar: BottleStableFileCellar,
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
}

#[derive(DeserializeFromStr)]
pub(in super::super) enum BottleStableFileCellar {
    Any,
    AnySkipRelocation,
    Path(PathBuf),
}

#[cfg(not(debug_assertions))]
use std::convert::Infallible;

impl FromStr for BottleStableFileCellar {
    #[cfg(debug_assertions)]
    type Err = !;

    #[cfg(not(debug_assertions))]
    type Err = Infallible;

    fn from_str(bottle_cellar: &str) -> Result<Self, Self::Err> {
        let bottle_cellar = match bottle_cellar {
            "any" => Self::Any,
            "any_skip_relocation" => Self::AnySkipRelocation,
            pstr => {
                let path = PathBuf::from(pstr);

                Self::Path(path)
            },
        };

        Ok(bottle_cellar)
    }
}
