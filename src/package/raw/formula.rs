use std::{collections::HashMap, path::PathBuf, str::FromStr};

use serde::{Deserialize, de::IgnoredAny};
use serde_with::DeserializeFromStr;

use super::{super::PackageExt, RawPackageExt};
use crate::util::macos::codename::Codename;

#[derive(Deserialize)]
pub(crate) struct RawFormula {
    pub(in super::super) name: String,
    pub(in super::super) versions: Versions,
    pub(in super::super) revision: u64,
    pub(in super::super) bottle: Bottle,
    pub(in super::super) keg_only: bool,

    requirements: Vec<Requirement>,
    dependencies: Vec<String>,
    uses_from_macos: Vec<UseFromMacos>,
    uses_from_macos_bounds: Vec<UseFromMacosBound>,
}

impl RawFormula {
    pub(crate) fn requirements(&self) -> &[Requirement] {
        &self.requirements
    }

    pub(crate) fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    pub(crate) fn uses_from_macos_bounds(
        &self,
    ) -> impl Iterator<Item = (&UseFromMacos, &UseFromMacosBound)> {
        assert_eq!(
            self.uses_from_macos.len(),
            self.uses_from_macos_bounds.len()
        );

        self.uses_from_macos
            .iter()
            .zip(self.uses_from_macos_bounds.iter())
    }
}

impl PackageExt for RawFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.versions.stable
    }
}

impl RawPackageExt for RawFormula {}

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
    AnySkipRelocator,
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
        let this = match bottle_cellar {
            "any" => Self::Any,
            "any_skip_relocation" => Self::AnySkipRelocator,
            bottle_cellar_pstr => {
                let bottle_cellar_path = PathBuf::from(bottle_cellar_pstr);

                Self::Path(bottle_cellar_path)
            },
        };

        Ok(this)
    }
}

#[derive(Deserialize)]
pub(crate) struct Requirement {
    pub(crate) name: RequirementName,
    pub(crate) version: Option<String>,
    pub(crate) contexts: Vec<IgnoredAny>,
    pub(crate) specs: Vec<RequirementSpec>,
}

#[derive(DeserializeFromStr)]
pub(crate) enum RequirementName {
    MinimumXcode,
    MinimumMacos,
    MaximumMacos,
    Linux,
    Arch,
    Unsupported(String),
}

impl FromStr for RequirementName {
    #[cfg(debug_assertions)]
    type Err = !;

    #[cfg(not(debug_assertions))]
    type Err = Infallible;

    fn from_str(requirement_name: &str) -> Result<Self, Self::Err> {
        let this = match requirement_name {
            "xcode" => Self::MinimumXcode,
            "macos" => Self::MinimumMacos,
            "maximum_macos" => Self::MaximumMacos,
            "linux" => Self::Linux,
            "arch" => Self::Arch,
            unsupported => Self::Unsupported(unsupported.to_owned()),
        };

        Ok(this)
    }
}

#[derive(PartialEq, Eq, Deserialize)]
pub(crate) enum RequirementSpec {
    #[serde(rename = "stable")]
    Stable,
    #[serde(rename = "head")]
    Head,
}

#[derive(Deserialize)]
pub(crate) enum DependencyType {
    #[serde(rename = "build")]
    Build,
    #[serde(rename = "test")]
    Test,
    #[serde(rename = "recommended")]
    Recommended,
    #[serde(rename = "optional")]
    Optional,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum UseFromMacos {
    Dependency(String),
    HashedDependencies(HashMap<String, UseFromMacosDependencyType>),
}

impl UseFromMacos {
    pub(crate) fn dependencies(&self) -> Vec<&str> {
        match self {
            Self::Dependency(dependency) => vec![dependency],
            Self::HashedDependencies(_) => Vec::new(),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum UseFromMacosDependencyType {
    Single(DependencyType),
    Multiple(Vec<DependencyType>),
}

#[derive(Deserialize)]
pub(crate) struct UseFromMacosBound {
    #[serde(default)]
    pub(crate) since: Option<Codename>,
}
