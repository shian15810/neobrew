use std::{cmp::Ordering, collections::HashMap, str::FromStr, sync::Arc};

use oci_client::config::Architecture;
use os_info::Version;
use serde::Deserialize;

use super::Packageable;

#[derive(Deserialize)]
pub(crate) struct RawFormula {
    name: String,
    versions: Versions,
    revision: u64,
    bottle: Bottle,
    dependencies: Vec<String>,
}

impl Packageable for RawFormula {
    fn id(&self) -> &str {
        &self.name
    }
}

impl RawFormula {
    pub(crate) fn dependencies(&self) -> &[String] {
        &self.dependencies
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

impl BottleStable {
    fn file(&self) -> Option<&BottleStableFile> {
        let tag = self.tag()?;

        let file = self.files.get(tag)?;

        Some(file)
    }

    #[cfg(target_os = "macos")]
    fn tag(&self) -> Option<&str> {
        let info = os_info::get();

        let version = info.version();

        let tag_version = MacosVersion::try_from(version).ok()?;

        let tag = self
            .files
            .keys()
            .filter_map(|tag| {
                tag.parse::<MacosVersion>()
                    .ok()
                    .map(|candidate_version| (tag.as_str(), candidate_version))
            })
            .filter(|(_, candidate_version)| {
                candidate_version.arch == tag_version.arch && candidate_version <= &tag_version
            })
            .max_by(|(_, x), (_, y)| x.cmp(y))
            .map(|(tag, _)| tag);

        self.tag_or_else(tag)
    }

    #[cfg(target_os = "linux")]
    fn tag(&self) -> Option<&str> {
        let tag = cfg_select! {
            target_arch = "aarch64" => "arm64_linux",
            target_arch = "x86_64" => "x86_64_linux",
        };
        let tag = self.files.contains_key(tag).then_some(tag);

        self.tag_or_else(tag)
    }

    fn tag_or_else<'a>(&self, tag: Option<&'a str>) -> Option<&'a str> {
        tag.or_else(|| self.files.contains_key("all").then_some("all"))
    }
}

#[derive(Deserialize)]
struct BottleStableFile {
    url: String,
    sha256: String,
}

struct MacosVersion {
    name: String,
    major: u64,
    minor: Option<u64>,
    arch: Architecture,
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

impl TryFrom<&Version> for MacosVersion {
    type Error = ();

    fn try_from(version: &Version) -> Result<Self, Self::Error> {
        let Version::Semantic(major, minor, _) = version else {
            return Err(());
        };

        let (name, major, minor) = match (major, minor) {
            (26, _) => ("tahoe", 26, None),
            (15, _) => ("sequoia", 15, None),
            (14, _) => ("sonoma", 14, None),
            (13, _) => ("ventura", 13, None),
            (12, _) => ("monterey", 12, None),
            (11, _) => ("big_sur", 11, None),
            (10, 15) => ("catalina", 10, Some(15)),
            _ => return Err(()),
        };

        let this = Self {
            name: name.to_owned(),
            major,
            minor,
            arch: Architecture::default(),
        };

        Ok(this)
    }
}

impl FromStr for MacosVersion {
    type Err = ();

    fn from_str(tag: &str) -> Result<Self, Self::Err> {
        let name = tag.strip_prefix("arm64_").unwrap_or(tag);

        let (major, minor) = match name {
            "tahoe" => (26, None),
            "sequoia" => (15, None),
            "sonoma" => (14, None),
            "ventura" => (13, None),
            "monterey" => (12, None),
            "big_sur" => (11, None),
            "catalina" => (10, Some(15)),
            _ => return Err(()),
        };

        let this = Self {
            name: name.to_owned(),
            major,
            minor,
            arch: Architecture::default(),
        };

        Ok(this)
    }
}
