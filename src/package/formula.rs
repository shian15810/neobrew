use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use base16ct::HexDisplay;
use oci_client::{Reference, manifest::OciDescriptor};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};

#[cfg(target_os = "macos")]
use self::macos::MacosTag;
use super::{
    Packageable,
    PreparedPackageFetchCache,
    PreparedPackageable,
    PreparedPackageableInner,
    RawPackageJsonCache,
    RawPackageable,
};
use crate::context::{Context, ProjectDirs as _};
#[cfg(target_os = "macos")]
use crate::ext::ResultExt as _;

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

        let cache_dir = context.homebrew_dirs.cache_dir();

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
    type Error = Option<anyhow::Error>;

    fn try_from(resolved_formula: ResolvedFormula) -> Result<Self, Self::Error> {
        let Some((tag, bottle_stable_file)) = resolved_formula.bottle.stable.entry()? else {
            return Err(None);
        };

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
    async fn fetch_cache(&self, context: &Context) -> Result<PreparedPackageFetchCache> {
        let fetch_cache = self.bottle_stable_file.fetch_cache(self, context);

        Ok(fetch_cache)
    }

    fn fetch_sha256(&self) -> &str {
        &self.bottle_stable_file.sha256
    }
}

impl PreparedPackageableInner for PreparedFormula {}

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
    fn entry(mut self) -> Result<Option<(String, BottleStableFile)>> {
        let Some(tag) = self.tag()? else {
            return Ok(None);
        };

        let Some(entry) = self.files.remove_entry(&tag) else {
            let err = anyhow!(r#"Computed bottle tag "{tag}" is missing from files"#);

            return Err(err);
        };

        Ok(Some(entry))
    }

    #[cfg(target_os = "macos")]
    fn tag(&self) -> Result<Option<String>> {
        let current_macos_tag = MacosTag::try_default()?;

        #[cfg(debug_assertions)]
        #[expect(clippy::redundant_iter_cloned)]
        let tagged_candidate_macos_tags = self
            .files
            .keys()
            .cloned()
            .map(|tag| tag.parse::<MacosTag>().map(|macos_tag| (tag, macos_tag)))
            .filter_map(Result::transpose_err)
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        #[expect(clippy::redundant_iter_cloned)]
        let tagged_candidate_macos_tags = self
            .files
            .keys()
            .cloned()
            .map(|tag| tag.parse::<MacosTag>().map(|macos_tag| (tag, macos_tag)))
            .filter_map(Result::transpose_err)
            .collect::<Result<Vec<_>>>()?;

        let tag = tagged_candidate_macos_tags
            .into_iter()
            .filter(|(_, candidate_macos_tag)| {
                candidate_macos_tag.architecture() == current_macos_tag.architecture()
                    && candidate_macos_tag <= &current_macos_tag
            })
            .max_by(|(_, left), (_, right)| left.cmp(right))
            .map(|(tag, _)| tag);

        let Some(tag) = tag.or_else(|| self.tag_or_else()) else {
            return Ok(None);
        };

        Ok(Some(tag))
    }

    #[cfg(target_os = "linux")]
    #[expect(clippy::unnecessary_wraps)]
    fn tag(&self) -> Result<Option<String>> {
        let tag = cfg_select! {
            target_arch = "aarch64" => "arm64_linux",
            target_arch = "x86_64" => "x86_64_linux",
        };
        let tag = self.files.contains_key(tag).then(|| tag.to_owned());

        let Some(tag) = tag.or_else(|| self.tag_or_else()) else {
            return Ok(None);
        };

        Ok(Some(tag))
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
        prepared_formula: &PreparedFormula,
        context: &Context,
    ) -> PreparedPackageFetchCache {
        let id = prepared_formula.id();

        let version = prepared_formula.version();

        let tag = &prepared_formula.tag;

        let url_hash = Sha256::digest(&self.url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let symlink_name = format!("{id}--{version}");

        let file_name = format!("{url_hash}--{symlink_name}.{tag}.bottle.tar.gz");

        let cache_dir = context.homebrew_dirs.cache_dir();

        let symlink_location_parent = cache_dir;

        prepared_formula.fetch_cache_inner(&file_name, &symlink_name, symlink_location_parent)
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

#[cfg(target_os = "macos")]
mod macos {
    use std::{cmp::Ordering, str::FromStr};

    use anyhow::{Result, anyhow};
    use oci_client::config::Architecture;
    use os_info::Version;

    struct MacosSemver {
        major: u64,
        minor: Option<u64>,
        patch: Option<u64>,
    }

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum MacosCodename {
        Catalina,
        BigSur,
        Monterey,
        Ventura,
        Sonoma,
        Sequoia,
        Tahoe,
    }

    impl TryFrom<MacosSemver> for MacosCodename {
        type Error = Option<anyhow::Error>;

        fn try_from(semver: MacosSemver) -> Result<Self, Self::Error> {
            let this = match (semver.major, semver.minor, semver.patch) {
                (26, ..) => Self::Tahoe,
                (15, ..) => Self::Sequoia,
                (14, ..) => Self::Sonoma,
                (13, ..) => Self::Ventura,
                (12, ..) => Self::Monterey,
                (11, ..) => Self::BigSur,
                (10, Some(15), _) => Self::Catalina,
                _ => return Err(None),
            };

            Ok(this)
        }
    }

    impl FromStr for MacosCodename {
        type Err = Option<anyhow::Error>;

        fn from_str(codename: &str) -> Result<Self, Self::Err> {
            let this = match codename {
                "tahoe" => Self::Tahoe,
                "sequoia" => Self::Sequoia,
                "sonoma" => Self::Sonoma,
                "ventura" => Self::Ventura,
                "monterey" => Self::Monterey,
                "big_sur" => Self::BigSur,
                "catalina" => Self::Catalina,
                _ => return Err(None),
            };

            Ok(this)
        }
    }

    #[derive(PartialEq, Eq)]
    pub(super) struct MacosTag {
        architecture: Architecture,
        codename: MacosCodename,
    }

    impl MacosTag {
        pub(super) fn architecture(&self) -> &Architecture {
            &self.architecture
        }

        pub(super) fn try_default() -> Result<Self> {
            let architecture = Architecture::default();

            let info = os_info::get();

            let version = info.version();

            let &Version::Semantic(major, minor, patch) = version else {
                let err = anyhow!(r#"Unsupported macOS version detected: "{version}""#);

                return Err(err);
            };

            let semver = MacosSemver {
                major,
                minor: Some(minor),
                patch: Some(patch),
            };

            let this = match Self::try_from((architecture, semver)) {
                Ok(this) => this,
                Err(Some(err)) => return Err(err),
                Err(None) => {
                    let err = anyhow!(r#"Unsupported macOS semver detected: "{version}""#);

                    return Err(err);
                },
            };

            Ok(this)
        }
    }

    impl TryFrom<(Architecture, MacosSemver)> for MacosTag {
        type Error = Option<anyhow::Error>;

        fn try_from(
            (architecture, semver): (Architecture, MacosSemver),
        ) -> Result<Self, Self::Error> {
            let codename = MacosCodename::try_from(semver)?;

            let this = Self {
                architecture,
                codename,
            };

            Ok(this)
        }
    }

    impl FromStr for MacosTag {
        type Err = Option<anyhow::Error>;

        fn from_str(tag: &str) -> Result<Self, Self::Err> {
            let (codename, architecture) = match tag.strip_prefix("arm64_") {
                Some(codename) => (codename, Architecture::ARM64),
                None => (tag, Architecture::Amd64),
            };

            let codename = codename.parse::<MacosCodename>()?;

            let this = Self {
                architecture,
                codename,
            };

            Ok(this)
        }
    }

    impl PartialOrd for MacosTag {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for MacosTag {
        fn cmp(&self, other: &Self) -> Ordering {
            self.architecture
                .to_string()
                .cmp(&other.architecture.to_string())
                .then_with(|| self.codename.cmp(&other.codename))
        }
    }
}
