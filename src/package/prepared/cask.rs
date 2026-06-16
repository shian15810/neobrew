use std::path::PathBuf;

use bytes::Bytes;
use futures::stream::BoxStream;
use tokio::fs;

use super::{
    super::{PackageExt, resolved::cask::ResolvedCask},
    PreparedPackageExt,
    cask_stanza::Stanzas,
    download::{Download, DownloadExt as _},
};
use crate::{context::Context, ext::tokio::path::PathExt as _};

pub(crate) struct PreparedCask<Dl = ()> {
    pub(in super::super) id: String,
    pub(in super::super) version: String,
    pub(super) url: String,
    pub(super) sha256: String,
    stanzas: Stanzas,
    is_compatible: bool,
    pub(in super::super) is_requested: bool,

    download: Dl,
}

impl TryFrom<(ResolvedCask, &Context)> for PreparedCask {
    type Error = anyhow::Error;

    fn try_from((resolved_cask, _context): (ResolvedCask, &Context)) -> Result<Self, Self::Error> {
        let this = Self {
            id: resolved_cask.id,
            version: resolved_cask.version,
            url: resolved_cask.url,
            sha256: resolved_cask.sha256,
            stanzas: Stanzas::from(resolved_cask.artifacts),
            is_compatible: resolved_cask.is_compatible.into_inner(),
            is_requested: resolved_cask.is_requested.into_inner(),

            download: (),
        };

        Ok(this)
    }
}

impl<Dl> From<(PreparedCask<()>, Dl)> for PreparedCask<Dl> {
    fn from((this, download): (PreparedCask<()>, Dl)) -> Self {
        Self {
            id: this.id,
            version: this.version,
            url: this.url,
            sha256: this.sha256,
            stanzas: this.stanzas,
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
        &self.id
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
        let id = &self.id;

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
        let id = &self.id;

        let version = &self.version;

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

    fn extract_dir_path(&self, context: &Context) -> PathBuf {
        let id = &self.id;

        let version = &self.version;

        context.homebrew_dirs.staged_dir(id, version)
    }
}

impl<Dl> PreparedCask<Dl> {
    pub(crate) fn stanzas(&self) -> &Stanzas {
        &self.stanzas
    }
}
