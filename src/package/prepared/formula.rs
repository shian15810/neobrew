use anyhow::{Result, anyhow};

use super::{
    super::{
        Formulable,
        Packageable,
        raw::{BottleStable, BottleStableFile},
        resolved::ResolvedFormula,
    },
    PreparedPackageable,
};

pub(crate) struct PreparedFormula {
    pub(in super::super) name: String,
    pub(in super::super) version_revision: String,
    bottle_rebuild: u64,
    bottle_tag: String,
    pub(in super::super) bottle_file: BottleStableFile,
    pub(in super::super) keg_only: bool,
}

impl TryFrom<ResolvedFormula> for PreparedFormula {
    type Error = Option<anyhow::Error>;

    fn try_from(resolved_formula: ResolvedFormula) -> Result<Self, Self::Error> {
        #[cfg(debug_assertions)]
        #[cfg_attr(
            debug_assertions,
            expect(resolving_to_items_shadowing_supertrait_items)
        )]
        let version_revision = {
            use super::super::resolved::ResolvedPackageable as _;

            resolved_formula.version()
        };

        #[cfg(not(debug_assertions))]
        let version_revision = {
            use super::super::resolved::ResolvedPackageable;

            ResolvedPackageable::version(&resolved_formula)
        };

        let version_revision = version_revision.into_owned();

        let bottle_rebuild = resolved_formula.bottle.stable.rebuild;

        let Some((bottle_tag, bottle_file)) = resolved_formula.bottle.stable.entry()? else {
            return Err(None);
        };

        let this = Self {
            name: resolved_formula.name,
            version_revision,
            bottle_rebuild,
            bottle_tag,
            bottle_file,
            keg_only: resolved_formula.keg_only,
        };

        Ok(this)
    }
}

impl Packageable for PreparedFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version_revision
    }
}

impl Formulable for PreparedFormula {
    fn keg_only(&self) -> bool {
        self.keg_only
    }
}

impl PreparedFormula {
    pub(crate) fn bottle_rebuild(&self) -> u64 {
        self.bottle_rebuild
    }

    pub(crate) fn bottle_tag(&self) -> &str {
        &self.bottle_tag
    }

    pub(crate) fn oci_url(&self) -> &str {
        &self.bottle_file.url
    }

    pub(crate) fn oci_sha256(&self) -> &str {
        &self.bottle_file.sha256
    }
}

impl PreparedPackageable for PreparedFormula {
    fn cache_url(&self) -> &str {
        &self.bottle_file.url
    }

    fn expected_sha256(&self) -> &str {
        &self.bottle_file.sha256
    }
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
        use crate::{ext::core::result::ResultExt as _, util::macos};

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
