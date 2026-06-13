use std::path::PathBuf;

use bytes::Bytes;
use futures::stream::BoxStream;
use tokio::fs;

use super::{
    super::{
        PackageExt,
        raw::cask::{Artifact, Variation},
        resolved::cask::ResolvedCask,
    },
    PreparedPackageExt,
    cask_stanza::Stanzas,
    download::{Download, DownloadExt as _},
};
use crate::{context::Context, ext::tokio::path::PathExt as _};

pub(crate) struct PreparedCask<Dl = ()> {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    variation_tag: Option<String>,
    variation_url: String,
    variation_sha256: String,
    variation_stanzas: Stanzas,
    is_compatible: bool,
    pub(in super::super) is_requested: bool,

    download: Dl,
}

impl TryFrom<ResolvedCask> for PreparedCask {
    type Error = anyhow::Error;

    fn try_from(mut resolved_cask: ResolvedCask) -> Result<Self, Self::Error> {
        let token = resolved_cask.token.clone();

        let version = resolved_cask.version.clone();

        let is_compatible = *resolved_cask.is_compatible.get_mut();

        let is_requested = *resolved_cask.is_requested.get_mut();

        let (variation_tag, variation) = resolved_cask.variation_entry()?;

        let this = Self {
            token,
            version,
            variation_tag,
            variation_url: variation.url,
            variation_sha256: variation.sha256,
            variation_stanzas: Stanzas::from(variation.artifacts),
            is_compatible,
            is_requested,

            download: (),
        };

        Ok(this)
    }
}

impl<Dl> From<(PreparedCask<()>, Dl)> for PreparedCask<Dl> {
    fn from((this, download): (PreparedCask<()>, Dl)) -> Self {
        Self {
            token: this.token,
            version: this.version,
            variation_tag: this.variation_tag,
            variation_url: this.variation_url,
            variation_sha256: this.variation_sha256,
            variation_stanzas: this.variation_stanzas,
            is_compatible: this.is_compatible,
            is_requested: this.is_requested,

            download,
        }
    }
}

impl PreparedCask<()> {
    pub(super) async fn with_download(
        self,
        context: &Context,
    ) -> anyhow::Result<(
        PreparedCask<Download>,
        BoxStream<'static, anyhow::Result<Bytes>>,
    )> {
        let (download, stream) = self.prepare_download(context).await?;

        let this = PreparedCask::from((self, download));

        Ok((this, stream))
    }
}

impl<Dl> PackageExt for PreparedCask<Dl> {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl<Dl> PreparedPackageExt for PreparedCask<Dl> {
    type Download = Dl;

    fn is_compatible(&self) -> bool {
        self.is_compatible
    }

    async fn is_installed(&self, context: &Context) -> anyhow::Result<bool> {
        let id = self.id();

        let cask_dir_path = context.homebrew_dirs.cask_dir(id);

        if !cask_dir_path.is_dir_exists_nofollow().await? {
            return Ok(false);
        }

        let mut cask_dir_entries = fs::read_dir(cask_dir_path).await?;

        while let Some(cask_dir_entry) = cask_dir_entries.next_entry().await? {
            let cask_dir_entry_path = cask_dir_entry.path();

            let is_cask_dir_entry_exists = cask_dir_entry_path.is_dir_exists_nofollow().await?;

            let is_cask_dir_entry_not_empty = !cask_dir_entry_path.is_dir_empty().await?;

            if is_cask_dir_entry_exists && is_cask_dir_entry_not_empty {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn is_up_to_date(&self, context: &Context) -> anyhow::Result<bool> {
        let id = self.id();

        let version = self.version();

        let staged_dir_path = context.homebrew_dirs.staged_dir(id, version);

        let is_staged_dir_exists = staged_dir_path.is_dir_exists_nofollow().await?;

        let is_staged_dir_not_empty = !staged_dir_path.is_dir_empty().await?;

        if is_staged_dir_exists && is_staged_dir_not_empty {
            return Ok(true);
        }

        Ok(false)
    }

    fn download(&self) -> &Self::Download {
        &self.download
    }

    fn pour_dir_path(&self, context: &Context) -> PathBuf {
        let id = self.id();

        let version = self.version();

        context.homebrew_dirs.staged_dir(id, version)
    }
}

impl<Dl> PreparedCask<Dl> {
    pub(super) fn variation_url(&self) -> &str {
        &self.variation_url
    }

    pub(super) fn variation_sha256(&self) -> &str {
        &self.variation_sha256
    }

    pub(crate) fn stanzas(&self) -> &Stanzas {
        &self.variation_stanzas
    }
}

impl ResolvedCask {
    fn variation_entry(mut self) -> anyhow::Result<(Option<String>, Variation<Vec<Artifact>>)> {
        #[expect(clippy::collapsible_if)]
        if let Some(tag) = self.variation_tag()? {
            if let Some((tag, variation)) = self.variations.remove_entry(&tag) {
                let variation = variation.unwrap_artifacts_or(self.artifacts);

                let entry = (Some(tag), variation);

                return Ok(entry);
            }
        }

        let variation = Variation {
            url: self.url,
            sha256: self.sha256,
            artifacts: self.artifacts,
        };

        let entry = (None, variation);

        Ok(entry)
    }

    #[cfg(target_os = "macos")]
    fn variation_tag(&self) -> anyhow::Result<Option<String>> {
        use crate::util::macos::tag::{Tag, TagError};

        let current_macos_tag = Tag::try_default()?;

        #[cfg(debug_assertions)]
        let tagged_candidate_macos_tags = self
            .variations
            .keys()
            .filter_map(|tag| {
                let macos_tag = match tag.parse::<Tag>() {
                    Ok(macos_tag) => macos_tag,
                    Err(TagError::Unsupported) => return None,
                    Err(TagError::Other(err)) => return Some(Err(err)),
                };

                Some(Ok((tag, macos_tag)))
            })
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let tagged_candidate_macos_tags = self
            .variations
            .keys()
            .filter_map(|tag| {
                let macos_tag = match tag.parse::<Tag>() {
                    Ok(macos_tag) => macos_tag,
                    Err(TagError::Unsupported) => return None,
                    Err(TagError::Other(err)) => return Some(Err(err)),
                };

                Some(Ok((tag, macos_tag)))
            })
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

        let Some(tag) = tag else {
            return Ok(None);
        };

        Ok(Some(tag))
    }

    #[cfg(target_os = "linux")]
    #[expect(clippy::unnecessary_wraps)]
    fn variation_tag(&self) -> anyhow::Result<Option<String>> {
        let tag = cfg_select! {
            target_arch = "aarch64" => "arm64_linux",
            target_arch = "x86_64" => "x86_64_linux",
        };
        let tag = self.variations.contains_key(tag).then(|| tag.to_owned());

        let Some(tag) = tag else {
            return Ok(None);
        };

        Ok(Some(tag))
    }
}

impl Variation {
    fn unwrap_artifacts_or(self, default: Vec<Artifact>) -> Variation<Vec<Artifact>> {
        #[cfg(debug_assertions)]
        let this = Variation {
            artifacts: self.artifacts.unwrap_or(default),

            ..self
        };

        #[cfg(not(debug_assertions))]
        let this = Variation {
            url: self.url,
            sha256: self.sha256,
            artifacts: self.artifacts.unwrap_or(default),
        };

        this
    }
}
