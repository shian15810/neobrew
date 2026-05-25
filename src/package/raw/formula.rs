use std::{borrow::Cow, collections::HashMap};

use serde::Deserialize;

use super::{
    super::{Formulable, Packageable},
    RawPackageable,
};

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

impl RawPackageable for RawFormula {
    fn version(&self) -> Cow<'_, str> {
        let version = &self.versions.stable;

        match self.revision {
            0 => Cow::Borrowed(version),
            revision => {
                let version_revision = format!("{version}_{revision}");

                Cow::Owned(version_revision)
            },
        }
    }
}

impl Formulable for RawFormula {
    fn keg_only(&self) -> bool {
        self.keg_only
    }
}

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

#[derive(PartialEq, Deserialize)]
pub(in super::super) enum BottleStableFileCellar {
    #[serde(rename = ":any")]
    Any,
    #[serde(rename = ":any_skip_relocation")]
    AnySkipRelocation,
}
