use anyhow::Context as _;

use super::{
    super::{
        Packageable,
        raw::{BottleStable, BottleStableFile, BottleStableFileCellar},
        resolved::ResolvedFormula,
    },
    PreparedPackageable,
};

pub(crate) struct PreparedFormula {
    pub(in super::super) name: String,
    pub(in super::super) version_revision: String,
    bottle_rebuild: u64,
    bottle_tag: String,
    pub(in super::super) bottle_cellar: BottleStableFileCellar,
    bottle_url: String,
    bottle_sha256: String,
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

        let Some((bottle_tag, bottle)) = resolved_formula.bottle.stable.entry()? else {
            return Err(None);
        };

        let this = Self {
            name: resolved_formula.name,
            version_revision,
            bottle_rebuild,
            bottle_tag,
            bottle_cellar: bottle.cellar,
            bottle_url: bottle.url,
            bottle_sha256: bottle.sha256,
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

impl PreparedFormula {
    pub(crate) fn bottle_rebuild(&self) -> u64 {
        self.bottle_rebuild
    }

    pub(crate) fn bottle_tag(&self) -> &str {
        &self.bottle_tag
    }

    pub(crate) fn oci_url(&self) -> &str {
        &self.bottle_url
    }

    pub(crate) fn oci_sha256(&self) -> &str {
        &self.bottle_sha256
    }

    pub(crate) fn should_link_keg(&self) -> bool {
        !self.keg_only
    }
}

impl PreparedPackageable for PreparedFormula {
    fn download_url(&self) -> &str {
        &self.bottle_url
    }

    fn expected_sha256(&self) -> &str {
        &self.bottle_sha256
    }
}

impl BottleStable {
    fn entry(mut self) -> anyhow::Result<Option<(String, BottleStableFile)>> {
        let Some(tag) = self.tag()? else {
            return Ok(None);
        };

        let entry = self
            .files
            .remove_entry(&tag)
            .with_context(|| format!(r#"Computed bottle tag "{tag}" is missing from files"#))?;

        Ok(Some(entry))
    }

    #[cfg(target_os = "macos")]
    fn tag(&self) -> anyhow::Result<Option<String>> {
        use crate::{ext::core::result::ResultExt as _, util::macos};

        let current_macos_tag = macos::Tag::try_default()?;

        #[cfg(debug_assertions)]
        let tagged_candidate_macos_tags = self
            .files
            .keys()
            .map(|tag| tag.parse::<macos::Tag>().map(|macos_tag| (tag, macos_tag)))
            .filter_map(Result::transpose_err)
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let tagged_candidate_macos_tags = self
            .files
            .keys()
            .map(|tag| tag.parse::<macos::Tag>().map(|macos_tag| (tag, macos_tag)))
            .filter_map(Result::transpose_err)
            .collect::<anyhow::Result<Vec<_>>>()?;

        let tag = tagged_candidate_macos_tags
            .into_iter()
            .filter(|(_, candidate_macos_tag)| {
                let is_macos_architecture_equal =
                    candidate_macos_tag.architecture() == current_macos_tag.architecture();

                is_macos_architecture_equal && candidate_macos_tag <= &current_macos_tag
            })
            .max_by(|(_, left), (_, right)| left.cmp(right))
            .map(|(tag, _)| tag.to_owned());

        let Some(tag) = tag.or_else(|| self.tag_or_else()) else {
            return Ok(None);
        };

        Ok(Some(tag))
    }

    #[cfg(target_os = "linux")]
    #[expect(clippy::unnecessary_wraps)]
    fn tag(&self) -> anyhow::Result<Option<String>> {
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
