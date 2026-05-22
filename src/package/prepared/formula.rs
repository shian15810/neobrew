use anyhow::{Result, anyhow};
use base16ct::HexDisplay;
use oci_client::{Reference, manifest::OciDescriptor};
use sha2::{Digest as _, Sha256};

use super::{
    super::{
        Packageable,
        raw::{BottleStable, BottleStableFile},
        resolved::ResolvedFormula,
    },
    PreparedPackageCache,
    PreparedPackageable,
    PreparedPackageableInner,
};
use crate::context::{Context, dirs::ProjectDirs as _};

pub(crate) struct PreparedFormula {
    pub(in super::super) name: String,
    pub(in super::super) version: String,
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
        let version = {
            use super::super::resolved::ResolvedPackageable as _;

            let version = resolved_formula.version();

            version.into_owned()
        };

        #[cfg(not(debug_assertions))]
        let version = {
            use super::super::resolved::ResolvedPackageable;

            let version = ResolvedPackageable::version(&resolved_formula);

            version.into_owned()
        };

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
    pub(super) fn oci(&self) -> Option<PreparedFormulaOci> {
        let oci = self.bottle_file.oci()?;

        Some(oci)
    }
}

pub(super) struct PreparedFormulaOci {
    pub(super) registry: &'static str,
    pub(super) reference: Reference,
    pub(super) descriptor: OciDescriptor,
}

impl PreparedFormulaOci {
    const REGISTRY: &str = "ghcr.io";
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
