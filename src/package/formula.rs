use std::{borrow::Cow, collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{Result, anyhow};
use base16ct::HexDisplay;
use oci_client::{Reference, manifest::OciDescriptor};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use walkdir::WalkDir;

use super::{
    Packageable,
    PreparedPackageCache,
    PreparedPackageDest,
    PreparedPackageable,
    PreparedPackageableInner,
    RawPackageCache,
    RawPackageable,
    ResolvedPackageable,
};
use crate::context::{Context, dirs::ProjectDirs as _};

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

    fn cache(&self, context: &Context) -> RawPackageCache {
        let id = self.id();

        let file_name = format!("{id}.json");

        let cache_dir = context.homebrew_dirs.cache_dir();

        let file_location_parent = cache_dir.join("api").join("formula");

        let file_location = file_location_parent.join(file_name);

        RawPackageCache {
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

impl ResolvedPackageable for ResolvedFormula {
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
    version: String,
    bottle_rebuild: u64,
    bottle_tag: String,
    bottle_file: BottleStableFile,
}

impl TryFrom<ResolvedFormula> for PreparedFormula {
    type Error = Option<anyhow::Error>;

    fn try_from(resolved_formula: ResolvedFormula) -> Result<Self, Self::Error> {
        #[cfg(debug_assertions)]
        #[cfg_attr(
            debug_assertions,
            expect(resolving_to_items_shadowing_supertrait_items)
        )]
        let version = resolved_formula.version().into_owned();

        #[cfg(not(debug_assertions))]
        let version = ResolvedPackageable::version(&resolved_formula).into_owned();

        let bottle_rebuild = resolved_formula.bottle.stable.rebuild;

        let Some((bottle_tag, bottle_file)) = resolved_formula.bottle.stable.entry()? else {
            return Err(None);
        };

        let this = Self {
            name: resolved_formula.name,
            version,
            bottle_rebuild,
            bottle_tag,
            bottle_file,
        };

        Ok(this)
    }
}

impl Packageable for PreparedFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl PreparedPackageable for PreparedFormula {
    async fn cache(&self, context: &Context) -> Result<PreparedPackageCache> {
        let cache = self.bottle_file.cache(self, context);

        Ok(cache)
    }

    fn sha256(&self) -> &str {
        &self.bottle_file.sha256
    }
}

impl PreparedPackageableInner for PreparedFormula {}

impl PreparedFormula {
    pub(crate) fn oci(&self) -> Option<PreparedFormulaOci> {
        let oci = self.bottle_file.oci()?;

        Some(oci)
    }
}

pub(crate) struct PreparedFormulaOci {
    pub(crate) registry: &'static str,
    pub(crate) reference: Reference,
    pub(crate) descriptor: OciDescriptor,
}

impl PreparedFormulaOci {
    const REGISTRY: &str = "ghcr.io";
}

pub(crate) struct FetchedFormula {
    name: String,
    version: String,
    prefix_dir: PathBuf,
    cellar_dir: PathBuf,
    rack_dir: PathBuf,
    keg_dir: PathBuf,
}

impl From<(PreparedFormula, PreparedPackageDest)> for FetchedFormula {
    fn from((prepared_formula, dest): (PreparedFormula, PreparedPackageDest)) -> Self {
        Self {
            name: prepared_formula.name,
            version: prepared_formula.version,
            prefix_dir: dest.dir_location_greatgrandparent,
            cellar_dir: dest.dir_location_grandparent,
            rack_dir: dest.dir_location_parent,
            keg_dir: dest.dir_location,
        }
    }
}

impl Packageable for FetchedFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl FetchedFormula {
    #[cfg(target_os = "macos")]
    pub(crate) fn relocate_keg(&self, context: &Context) -> Result<()> {
        use crate::os::macos::{Codesign, Relocation};

        let relocation = Relocation::from(&context.homebrew_dirs);

        for entry in WalkDir::new(&self.keg_dir) {
            let entry = entry?;

            let path = entry.path();

            relocation.patch_file(path)?;

            Codesign::sign_in_place(path)?;
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn relocate_keg(&self, _context: &Context) -> Result<()> {
        for entry in WalkDir::new(&self.keg_dir) {
            let entry = entry?;

            let _path = entry.path();
        }

        Ok(())
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
        use crate::{ext::ResultExt as _, os::macos};

        let current_macos_tag = macos::Tag::try_default()?;

        #[cfg(debug_assertions)]
        #[expect(clippy::redundant_iter_cloned)]
        let tagged_candidate_macos_tags = self
            .files
            .keys()
            .cloned()
            .map(|tag| tag.parse::<macos::Tag>().map(|macos_tag| (tag, macos_tag)))
            .filter_map(Result::transpose_err)
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        #[expect(clippy::redundant_iter_cloned)]
        let tagged_candidate_macos_tags = self
            .files
            .keys()
            .cloned()
            .map(|tag| tag.parse::<macos::Tag>().map(|macos_tag| (tag, macos_tag)))
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
    fn cache(&self, prepared_formula: &PreparedFormula, context: &Context) -> PreparedPackageCache {
        let id = prepared_formula.id();

        let version = prepared_formula.version();

        let bottle_tag = &prepared_formula.bottle_tag;

        let url_hash = Sha256::digest(&self.url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let symlink_name = format!("{id}--{version}");

        let file_name = match prepared_formula.bottle_rebuild {
            0 => format!("{url_hash}--{symlink_name}.{bottle_tag}.bottle.tar.gz"),
            bottle_rebuild => {
                format!("{url_hash}--{symlink_name}.{bottle_tag}.bottle.{bottle_rebuild}.tar.gz")
            },
        };

        let cache_dir = context.homebrew_dirs.cache_dir();

        let symlink_location_parent = cache_dir;

        prepared_formula.cache_inner(&file_name, &symlink_name, symlink_location_parent)
    }

    fn oci(&self) -> Option<PreparedFormulaOci> {
        let registry = PreparedFormulaOci::REGISTRY;

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

        let oci = PreparedFormulaOci {
            registry,
            reference,
            descriptor,
        };

        Some(oci)
    }
}
