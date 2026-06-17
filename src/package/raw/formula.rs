use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, de::IgnoredAny};
use serde_with::DeserializeFromStr;
use strum::EnumString;

use super::{super::PackageExt, RawPackageExt};
use crate::{context::Context, util::macos::codename::Codename};

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

    variations: HashMap<String, Variation>,
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
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
    pub(in super::super) cellar: BottleStableFileCellar,
}

#[derive(Debug, EnumString, DeserializeFromStr)]
pub(in super::super) enum BottleStableFileCellar {
    #[strum(to_string = ":any")]
    Any,
    #[strum(to_string = ":any_skip_relocation")]
    AnySkipRelocator,
    #[strum(default)]
    Path(PathBuf),
}

#[derive(Deserialize)]
pub(crate) struct Requirement {
    pub(crate) name: RequirementName,
    pub(crate) version: Option<String>,
    pub(crate) contexts: Vec<IgnoredAny>,
    pub(crate) specs: Vec<RequirementSpec>,
}

#[derive(Debug, EnumString, DeserializeFromStr)]
pub(crate) enum RequirementName {
    #[strum(to_string = "xcode")]
    MinimumXcode,
    #[strum(to_string = "macos")]
    MinimumMacos,
    #[strum(to_string = "maximum_macos")]
    MaximumMacos,
    #[strum(to_string = "linux")]
    Linux,
    #[strum(to_string = "arch")]
    Arch,
    #[strum(default)]
    Unsupported(String),
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

#[derive(Deserialize)]
struct Variation {
    keg_only: Option<bool>,

    requirements: Option<Vec<Requirement>>,

    dependencies: Option<Vec<String>>,
}

impl RawFormula {
    pub(crate) fn squash_variations(mut self, context: &Context) -> anyhow::Result<Self> {
        #[expect(clippy::collapsible_if)]
        if let Some(variation_key) = self.variation_key(context)? {
            if let Some(variation) = self.variations.remove(&variation_key) {
                if let Some(keg_only) = variation.keg_only {
                    self.keg_only = keg_only;
                }

                if let Some(requirements) = variation.requirements {
                    self.requirements = requirements;
                }

                if let Some(dependencies) = variation.dependencies {
                    self.dependencies = dependencies;
                }
            }
        }

        self.variations.clear();

        self.variations.shrink_to_fit();

        Ok(self)
    }

    #[cfg(target_os = "macos")]
    fn variation_key(&self, context: &Context) -> anyhow::Result<Option<String>> {
        use crate::util::macos::tag::{Tag, TagError};

        let current_tag = Tag::try_default(context)?;

        #[cfg(debug_assertions)]
        let variation_keys_tags = self
            .variations
            .keys()
            .filter_map(|variation_key| {
                let variation_tag = match variation_key.parse::<Tag>() {
                    Ok(variation_tag) => variation_tag,
                    Err(TagError::Unsupported) => return None,
                    Err(TagError::Other(err)) => return Some(Err(err)),
                };

                Some(Ok((variation_key, variation_tag)))
            })
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let variation_keys_tags = self
            .variations
            .keys()
            .filter_map(|variation_key| {
                let variation_tag = match variation_key.parse::<Tag>() {
                    Ok(variation_tag) => variation_tag,
                    Err(TagError::Unsupported) => return None,
                    Err(TagError::Other(err)) => return Some(Err(err)),
                };

                Some(Ok((variation_key, variation_tag)))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let variation_key = variation_keys_tags
            .into_iter()
            .filter(|(_, variation_tag)| {
                let is_macos_architecture_equal =
                    variation_tag.architecture() == current_tag.architecture();

                is_macos_architecture_equal && variation_tag <= &current_tag
            })
            .max_by(|(_, left), (_, right)| left.cmp(right))
            .map(|(variation_key, _)| variation_key.to_owned());

        Ok(variation_key)
    }

    #[cfg(target_os = "linux")]
    #[expect(clippy::unnecessary_wraps)]
    fn variation_key(&self, _context: &Context) -> anyhow::Result<Option<String>> {
        let variation_key = cfg_select! {
            target_arch = "aarch64" => "arm64_linux",
            target_arch = "x86_64" => "x86_64_linux",
        };
        let variation_key = self
            .variations
            .contains_key(variation_key)
            .then(|| variation_key.to_owned());

        Ok(variation_key)
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use serde_json::json;

    use super::*;

    #[test]
    fn deserialize_bottle_stable_file_cellar() {
        let json = json!(":any");

        let cellar: BottleStableFileCellar = serde_json::from_value(json).unwrap();

        assert_matches!(cellar, BottleStableFileCellar::Any);

        let json = json!(":any_skip_relocation");

        let cellar: BottleStableFileCellar = serde_json::from_value(json).unwrap();

        assert_matches!(cellar, BottleStableFileCellar::AnySkipRelocator);

        let json = json!("/usr/local/Cellar");

        let cellar: BottleStableFileCellar = serde_json::from_value(json).unwrap();

        assert_matches!(
            cellar,
            BottleStableFileCellar::Path(path) if path == *"/usr/local/Cellar",
        );
    }

    #[test]
    fn deserialize_requirement_name() {
        let json = json!("xcode");

        let name: RequirementName = serde_json::from_value(json).unwrap();

        assert_matches!(name, RequirementName::MinimumXcode);

        let json = json!("arch");

        let name: RequirementName = serde_json::from_value(json).unwrap();

        assert_matches!(name, RequirementName::Arch);

        let json = json!("linuxkernel");

        let name: RequirementName = serde_json::from_value(json).unwrap();

        assert_matches!(
            name,
            RequirementName::Unsupported(unsupported) if unsupported == "linuxkernel",
        );
    }
}
