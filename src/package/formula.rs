use std::{cmp::Ordering, collections::HashMap, str::FromStr, sync::Arc};

use anyhow::{Context as _, Result, anyhow};
use base16ct::HexDisplay;
use oci_client::{Reference, config::Architecture, manifest::OciDescriptor};
use os_info::Version;
use pathdiff::diff_paths;
use serde::Deserialize;
use sha2::{Digest as _, Sha256};

use super::{
    Packageable,
    PreparedPackageFetchCache,
    PreparedPackageable,
    RawPackageJsonCache,
    RawPackageable,
};
use crate::Context;

#[derive(Deserialize)]
pub(crate) struct RawFormula {
    name: String,
    versions: Versions,
    revision: u64,
    bottle: Bottle,
    dependencies: Vec<String>,
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
    fn json_cache(&self, context: &Context) -> RawPackageJsonCache {
        let id = self.id();

        let file_name = format!("{id}.json");

        let cache_dir = cfg_select! {
            debug_assertions => context.neobrew_dirs.cache_dir(),
            _ => context.homebrew_dirs.cache_dir(),
        };

        let file_location_parent = cache_dir.join("api").join("formula");

        let file_location = file_location_parent.join(file_name);

        RawPackageJsonCache {
            file_location_parent,
            file_location,
        }
    }
}

pub(crate) struct ResolvedFormula {
    name: String,
    versions: Versions,
    revision: u64,
    bottle: Bottle,
    dependencies: Vec<Arc<Self>>,
}

impl From<(RawFormula, Vec<Arc<Self>>)> for ResolvedFormula {
    fn from((raw_formula, this_dependencies): (RawFormula, Vec<Arc<Self>>)) -> Self {
        Self {
            name: raw_formula.name,
            versions: raw_formula.versions,
            revision: raw_formula.revision,
            bottle: raw_formula.bottle,
            dependencies: this_dependencies,
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

impl ResolvedFormula {
    pub(super) fn iter(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> + use<> {
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

pub(crate) struct PreparedFormula {
    name: String,
    versions: Versions,
    revision: u64,
    tag: String,
    bottle_stable_file: BottleStableFile,
}

impl TryFrom<ResolvedFormula> for PreparedFormula {
    type Error = anyhow::Error;

    fn try_from(resolved_formula: ResolvedFormula) -> Result<Self, Self::Error> {
        let (tag, bottle_stable_file) = resolved_formula
            .bottle
            .stable
            .entry()
            .context("Unexpected `None`")?;

        let this = Self {
            name: resolved_formula.name,
            versions: resolved_formula.versions,
            revision: resolved_formula.revision,
            tag,
            bottle_stable_file,
        };

        Ok(this)
    }
}

impl Packageable for PreparedFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.versions.stable
    }
}

impl PreparedPackageable for PreparedFormula {
    fn fetch_sha256(&self) -> &str {
        &self.bottle_stable_file.sha256
    }

    fn fetch_cache(&self, context: &Context) -> Option<PreparedPackageFetchCache> {
        let fetch_cache =
            self.bottle_stable_file
                .fetch_cache(self.id(), self.version(), &self.tag, context)?;

        Some(fetch_cache)
    }
}

impl PreparedFormula {
    pub(crate) fn fetch_oci(&self) -> Option<PreparedFormulaFetchOci> {
        let fetch_oci = self.bottle_stable_file.fetch_oci()?;

        Some(fetch_oci)
    }
}

pub(crate) struct PreparedFormulaFetchOci {
    pub(crate) registry: &'static str,
    pub(crate) reference: Reference,
    pub(crate) descriptor: OciDescriptor,
}

impl PreparedFormulaFetchOci {
    const REGISTRY: &str = "ghcr.io";
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

impl BottleStable {
    fn entry(mut self) -> Option<(String, BottleStableFile)> {
        let tag = self.tag()?;

        let entry = self.files.remove_entry(&tag)?;

        Some(entry)
    }

    #[cfg(target_os = "macos")]
    fn tag(&self) -> Option<String> {
        let info = os_info::get();

        let version = info.version();

        let tag_version = MacosVersion::try_from(version).ok()?;

        let tag = self
            .files
            .keys()
            .cloned()
            .filter_map(|tag| {
                let candidate_version: MacosVersion = tag.parse().ok()?;

                Some((tag, candidate_version))
            })
            .filter(|(_, candidate_version)| {
                candidate_version.architecture == tag_version.architecture
                    && candidate_version <= &tag_version
            })
            .max_by(|(_, left), (_, right)| left.cmp(right))
            .map(|(tag, _)| tag);
        let tag = tag.or_else(|| self.tag_or_else())?;

        Some(tag)
    }

    #[cfg(target_os = "linux")]
    fn tag(&self) -> Option<String> {
        let tag = cfg_select! {
            target_arch = "aarch64" => "arm64_linux",
            target_arch = "x86_64" => "x86_64_linux",
        };
        let tag = self.files.contains_key(tag).then(|| tag.to_owned());
        let tag = tag.or_else(|| self.tag_or_else())?;

        Some(tag)
    }

    fn tag_or_else(&self) -> Option<String> {
        let tag = "all".to_owned();
        let tag = self.files.contains_key(&tag).then_some(tag)?;

        Some(tag)
    }
}

#[derive(Deserialize)]
struct BottleStableFile {
    url: String,
    sha256: String,
}

impl BottleStableFile {
    fn fetch_cache(
        &self,
        id: &str,
        version: &str,
        tag: &str,
        context: &Context,
    ) -> Option<PreparedPackageFetchCache> {
        let url_hash = Sha256::digest(&self.url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let symlink_name = format!("{id}--{version}");

        let file_name = format!("{url_hash}--{symlink_name}.{tag}.bottle.tar.gz");

        let cache_dir = cfg_select! {
            debug_assertions => context.neobrew_dirs.cache_dir(),
            _ => context.homebrew_dirs.cache_dir(),
        };

        let symlink_location_parent = cache_dir;

        let file_location_parent = symlink_location_parent.join("downloads");

        let file_location = file_location_parent.join(file_name);

        let symlink_location_diff = diff_paths(&file_location, &symlink_location_parent)?;

        let symlink_location = symlink_location_parent.join(symlink_name);

        let symlink_location_tmp = symlink_location.with_extension("tmp");

        let cache = PreparedPackageFetchCache {
            file_location_parent,
            file_location,

            symlink_location_diff,
            symlink_location_tmp,
            symlink_location,
        };

        Some(cache)
    }

    fn fetch_oci(&self) -> Option<PreparedFormulaFetchOci> {
        let registry = PreparedFormulaFetchOci::REGISTRY;

        let repository = format!("https://{registry}/v2/");
        let repository = self.url.strip_prefix(&repository)?;
        let repository = repository.split("/blobs/").next()?;

        let sha256 = &self.sha256;

        let digest = format!("sha256:{sha256}");

        let reference =
            Reference::with_digest(registry.to_owned(), repository.to_owned(), digest.clone());

        let descriptor = OciDescriptor {
            digest,

            ..OciDescriptor::default()
        };

        let fetch_oci = PreparedFormulaFetchOci {
            registry,
            reference,
            descriptor,
        };

        Some(fetch_oci)
    }
}

struct MacosVersion {
    name: String,
    major: u64,
    minor: Option<u64>,
    architecture: Architecture,
}

impl TryFrom<&Version> for MacosVersion {
    type Error = anyhow::Error;

    fn try_from(version: &Version) -> Result<Self, Self::Error> {
        let Version::Semantic(major, minor, _) = version else {
            let err = anyhow!("Unexpected `Version`");

            return Err(err);
        };

        let (name, major, minor) = match (major, minor) {
            (26, _) => ("tahoe", 26, None),
            (15, _) => ("sequoia", 15, None),
            (14, _) => ("sonoma", 14, None),
            (13, _) => ("ventura", 13, None),
            (12, _) => ("monterey", 12, None),
            (11, _) => ("big_sur", 11, None),
            (10, 15) => ("catalina", 10, Some(15)),
            _ => {
                let err = anyhow!("Your macOS version is not supported");

                return Err(err);
            },
        };

        let this = Self {
            name: name.to_owned(),
            major,
            minor,
            architecture: Architecture::default(),
        };

        Ok(this)
    }
}

impl FromStr for MacosVersion {
    type Err = anyhow::Error;

    fn from_str(tag: &str) -> Result<Self, Self::Err> {
        let (name, architecture) = if let Some(name) = tag.strip_prefix("arm64_") {
            (name, Architecture::ARM64)
        } else {
            (tag, Architecture::Amd64)
        };

        let (major, minor) = match name {
            "tahoe" => (26, None),
            "sequoia" => (15, None),
            "sonoma" => (14, None),
            "ventura" => (13, None),
            "monterey" => (12, None),
            "big_sur" => (11, None),
            "catalina" => (10, Some(15)),
            _ => {
                let err = anyhow!("Unsupported macOS version detected");

                return Err(err);
            },
        };

        let this = Self {
            name: name.to_owned(),
            major,
            minor,
            architecture,
        };

        Ok(this)
    }
}

impl Eq for MacosVersion {}

impl PartialEq for MacosVersion {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major && self.minor == other.minor
    }
}

impl Ord for MacosVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
    }
}

impl PartialOrd for MacosVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
