use std::borrow::Cow;

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
    pub(in super::super) version: String,
    pub(in super::super) version_revision: String,
    bottle_rebuild: u64,
    bottle_tag: String,
    pub(in super::super) bottle_cellar: BottleStableFileCellar,
    bottle_url: String,
    bottle_sha256: String,
    pub(in super::super) keg_only: bool,
    pub(in super::super) is_requested: bool,
}

impl TryFrom<(ResolvedFormula, bool)> for PreparedFormula {
    type Error = Option<anyhow::Error>;

    fn try_from(
        (resolved_formula, is_requested): (ResolvedFormula, bool),
    ) -> Result<Self, Self::Error> {
        let version_revision = resolved_formula.version_revision();
        let version_revision = version_revision.into_owned();

        let bottle_rebuild = resolved_formula.bottle.stable.rebuild;

        let Some((bottle_tag, bottle)) = resolved_formula.bottle.stable.entry()? else {
            return Err(None);
        };

        let this = Self {
            name: resolved_formula.name,
            version: resolved_formula.versions.stable,
            version_revision,
            bottle_rebuild,
            bottle_tag,
            bottle_cellar: bottle.cellar,
            bottle_url: bottle.url,
            bottle_sha256: bottle.sha256,
            keg_only: resolved_formula.keg_only,
            is_requested,
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
    fn download_url(&self) -> &str {
        &self.bottle_url
    }

    fn expected_sha256(&self) -> &str {
        &self.bottle_sha256
    }
}

impl PreparedFormula {
    pub(crate) fn version_revision(&self) -> &str {
        &self.version_revision
    }

    pub(crate) fn bottle_rebuild(&self) -> u64 {
        self.bottle_rebuild
    }

    pub(crate) fn bottle_tag(&self) -> &str {
        &self.bottle_tag
    }

    pub(crate) fn should_link_keg(&self) -> bool {
        !self.keg_only
    }
}

impl ResolvedFormula {
    fn version_revision(&self) -> Cow<'_, str> {
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
