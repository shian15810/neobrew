use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use bytes::Bytes;
use futures::stream::BoxStream;
use tokio::fs;

use super::{
    super::{
        Packageable,
        raw::{BottleStable, BottleStableFile, BottleStableFileCellar},
        resolved::ResolvedFormula,
    },
    PreparedPackageable,
    download::{Download, Downloadable as _},
};
use crate::{context::Context, ext::tokio::path::PathExt as _};

pub(crate) struct PreparedFormula<Dl = ()> {
    pub(in super::super) name: String,
    pub(in super::super) version: String,
    version_revision: String,
    bottle_rebuild: u64,
    bottle_tag: String,
    bottle_cellar: BottleStableFileCellar,
    bottle_url: String,
    bottle_sha256: String,
    keg_only: bool,
    pub(in super::super) is_requested: bool,

    download: Dl,
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

            download: (),
        };

        Ok(this)
    }
}

impl<Dl> From<(PreparedFormula<()>, Dl)> for PreparedFormula<Dl> {
    fn from((this, download): (PreparedFormula<()>, Dl)) -> Self {
        Self {
            name: this.name,
            version: this.version,
            version_revision: this.version_revision,
            bottle_rebuild: this.bottle_rebuild,
            bottle_tag: this.bottle_tag,
            bottle_cellar: this.bottle_cellar,
            bottle_url: this.bottle_url,
            bottle_sha256: this.bottle_sha256,
            keg_only: this.keg_only,
            is_requested: this.is_requested,

            download,
        }
    }
}

impl PreparedFormula<()> {
    pub(super) async fn with_download(
        self,
        context: &Context,
    ) -> anyhow::Result<(
        PreparedFormula<Download>,
        BoxStream<'static, anyhow::Result<Bytes>>,
    )> {
        let (download, stream) = self.prepare_download(context).await?;

        let this = PreparedFormula::from((self, download));

        Ok((this, stream))
    }
}

impl<Dl> Packageable for PreparedFormula<Dl> {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl<Dl> PreparedPackageable for PreparedFormula<Dl> {
    type Download = Dl;

    fn download(&self) -> &Self::Download {
        &self.download
    }

    fn pour_dir_path(&self, context: &Context) -> PathBuf {
        context.homebrew_dirs.cellar_dir()
    }

    async fn is_installed(&self, context: &Context) -> anyhow::Result<bool> {
        let id = self.id();

        let rack_dir_path = context.homebrew_dirs.rack_dir(id);

        if !rack_dir_path.is_dir_exists_nofollow().await? {
            return Ok(false);
        }

        let mut rack_dir_entries = fs::read_dir(rack_dir_path).await?;

        while let Some(rack_dir_entry) = rack_dir_entries.next_entry().await? {
            let rack_dir_entry_path = rack_dir_entry.path();

            let is_rack_dir_entry_exists = rack_dir_entry_path.is_dir_exists_nofollow().await?;

            let is_rack_dir_entry_not_empty = !rack_dir_entry_path.is_dir_empty().await?;

            if is_rack_dir_entry_exists && is_rack_dir_entry_not_empty {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn is_up_to_date(&self, context: &Context) -> anyhow::Result<bool> {
        let id = self.id();

        let version_revision = self.version_revision();

        let keg_dir_path = context.homebrew_dirs.keg_dir(id, version_revision);

        let is_keg_dir_exists = keg_dir_path.is_dir_exists_nofollow().await?;

        let is_keg_dir_not_empty = !keg_dir_path.is_dir_empty().await?;

        if is_keg_dir_exists && is_keg_dir_not_empty {
            return Ok(true);
        }

        Ok(false)
    }
}

impl<Dl> PreparedFormula<Dl> {
    pub(crate) fn version_revision(&self) -> &str {
        &self.version_revision
    }

    pub(super) fn bottle_rebuild(&self) -> u64 {
        self.bottle_rebuild
    }

    pub(super) fn bottle_tag(&self) -> &str {
        &self.bottle_tag
    }

    pub(super) fn bottle_url(&self) -> &str {
        &self.bottle_url
    }

    pub(super) fn bottle_sha256(&self) -> &str {
        &self.bottle_sha256
    }

    pub(crate) fn should_relocate(&self, cellar_dir_path: &Path) -> bool {
        match &self.bottle_cellar {
            BottleStableFileCellar::Any => true,
            BottleStableFileCellar::AnySkipRelocator => false,
            BottleStableFileCellar::Path(bottle_cellar_path) => {
                bottle_cellar_path == cellar_dir_path
            },
        }
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
