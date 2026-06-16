use std::{borrow::Cow, path::PathBuf};

use anyhow::{Context as _, anyhow};
use bytes::Bytes;
use futures::stream::BoxStream;
use tokio::fs;

use super::{
    super::{
        PackageExt,
        raw::formula::{BottleStable, BottleStableFile, BottleStableFileCellar},
        resolved::formula::ResolvedFormula,
    },
    PreparedPackageExt,
    download::{Download, DownloadExt as _},
};
use crate::{context::Context, ext::tokio::path::PathExt as _};

pub(crate) struct PreparedFormula<Dl = ()> {
    pub(in super::super) id: String,
    pub(in super::super) version: String,
    revision: u64,
    pub(super) rebuild: u64,
    pub(super) bottle: String,
    pub(super) url: String,
    pub(super) sha256: String,
    cellar: BottleStableFileCellar,
    keg_only: bool,
    is_compatible: bool,
    pub(in super::super) is_requested: bool,

    download: Dl,
}

impl TryFrom<(ResolvedFormula, &Context)> for PreparedFormula {
    type Error = anyhow::Error;

    fn try_from(
        (resolved_formula, context): (ResolvedFormula, &Context),
    ) -> Result<Self, Self::Error> {
        let rebuild = resolved_formula.bottle.stable.rebuild;

        let Some((bottle, file)) = resolved_formula.bottle.stable.file(context)? else {
            let id = resolved_formula.id;

            let err = anyhow!(r#"Formula "{id}" has no bottle to download"#);

            return Err(err);
        };

        let this = Self {
            id: resolved_formula.id,
            version: resolved_formula.versions.stable,
            revision: resolved_formula.revision,
            rebuild,
            bottle,
            url: file.url,
            sha256: file.sha256,
            cellar: file.cellar,
            keg_only: resolved_formula.keg_only,
            is_compatible: resolved_formula.is_compatible.into_inner(),
            is_requested: resolved_formula.is_requested.into_inner(),

            download: (),
        };

        Ok(this)
    }
}

impl<Dl> From<(PreparedFormula<()>, Dl)> for PreparedFormula<Dl> {
    fn from((this, download): (PreparedFormula<()>, Dl)) -> Self {
        Self {
            id: this.id,
            version: this.version,
            revision: this.revision,
            rebuild: this.rebuild,
            bottle: this.bottle,
            url: this.url,
            sha256: this.sha256,
            cellar: this.cellar,
            keg_only: this.keg_only,
            is_compatible: this.is_compatible,
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

impl<Dl> PackageExt for PreparedFormula<Dl> {
    fn id(&self) -> &str {
        &self.id
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl<Dl> PreparedPackageExt for PreparedFormula<Dl> {
    type Download = Dl;

    fn is_compatible(&self) -> bool {
        self.is_compatible
    }

    async fn is_installed(&self, context: &Context) -> anyhow::Result<bool> {
        let id = &self.id;

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
        let id = &self.id;

        let version_revision = self.version_revision();

        let keg_dir_path = context.homebrew_dirs.keg_dir(id, &version_revision);

        let is_keg_dir_exists = keg_dir_path.is_dir_exists_nofollow().await?;

        let is_keg_dir_not_empty = !keg_dir_path.is_dir_empty().await?;

        if is_keg_dir_exists && is_keg_dir_not_empty {
            return Ok(true);
        }

        Ok(false)
    }

    fn download(&self) -> &Self::Download {
        &self.download
    }

    fn extract_dir_path(&self, context: &Context) -> PathBuf {
        context.homebrew_dirs.cellar_dir()
    }
}

impl<Dl> PreparedFormula<Dl> {
    pub(crate) fn version_revision(&self) -> Cow<'_, str> {
        let version = &self.version;

        match self.revision {
            0 => Cow::Borrowed(version),
            revision => {
                let version_revision = format!("{version}_{revision}");

                Cow::Owned(version_revision)
            },
        }
    }

    pub(crate) fn should_relocate(&self, context: &Context) -> bool {
        let extract_dir_path = self.extract_dir_path(context);

        match &self.cellar {
            BottleStableFileCellar::Any => true,
            BottleStableFileCellar::AnySkipRelocator => false,
            BottleStableFileCellar::Path(path) => path == &extract_dir_path,
        }
    }

    pub(crate) fn should_link_keg(&self) -> bool {
        !self.keg_only
    }
}

impl BottleStable {
    fn file(mut self, context: &Context) -> anyhow::Result<Option<(String, BottleStableFile)>> {
        let Some(file_key) = self.file_key(context)? else {
            return Ok(None);
        };

        let file = self
            .files
            .remove_entry(&file_key)
            .context("Computed key is missing from `bottle.stable.files` of formula")?;

        Ok(Some(file))
    }

    #[cfg(target_os = "macos")]
    fn file_key(&self, context: &Context) -> anyhow::Result<Option<String>> {
        use crate::util::macos::tag::{Tag, TagError};

        let current_tag = Tag::try_default(context)?;

        #[cfg(debug_assertions)]
        let file_keys_tags = self
            .files
            .keys()
            .filter_map(|file_key| {
                let file_tag = match file_key.parse::<Tag>() {
                    Ok(file_tag) => file_tag,
                    Err(TagError::Unsupported) => return None,
                    Err(TagError::Other(err)) => return Some(Err(err)),
                };

                Some(Ok((file_key, file_tag)))
            })
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let file_keys_tags = self
            .files
            .keys()
            .filter_map(|file_key| {
                let file_tag = match file_key.parse::<Tag>() {
                    Ok(file_tag) => file_tag,
                    Err(TagError::Unsupported) => return None,
                    Err(TagError::Other(err)) => return Some(Err(err)),
                };

                Some(Ok((file_key, file_tag)))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let file_key = file_keys_tags
            .into_iter()
            .filter(|(_, file_tag)| {
                let is_macos_architecture_equal =
                    file_tag.architecture() == current_tag.architecture();

                is_macos_architecture_equal && file_tag <= &current_tag
            })
            .max_by(|(_, left), (_, right)| left.cmp(right))
            .map(|(file_key, _)| file_key.to_owned());

        let Some(file_key) = file_key.or_else(|| self.file_key_or_else()) else {
            return Ok(None);
        };

        Ok(Some(file_key))
    }

    #[cfg(target_os = "linux")]
    #[expect(clippy::unnecessary_wraps)]
    fn file_key(&self, _context: &Context) -> anyhow::Result<Option<String>> {
        let file_key = cfg_select! {
            target_arch = "aarch64" => "arm64_linux",
            target_arch = "x86_64" => "x86_64_linux",
        };
        let file_key = self
            .files
            .contains_key(file_key)
            .then(|| file_key.to_owned());

        let Some(file_key) = file_key.or_else(|| self.file_key_or_else()) else {
            return Ok(None);
        };

        Ok(Some(file_key))
    }

    fn file_key_or_else(&self) -> Option<String> {
        let file_key = "all".to_owned();
        let file_key = self.files.contains_key(&file_key).then_some(file_key)?;

        Some(file_key)
    }
}
