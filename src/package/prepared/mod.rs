pub(crate) mod cask;
#[cfg(target_os = "macos")]
pub(crate) mod cask_stanza;
pub(crate) mod download;
pub(crate) mod formula;

use std::{path::PathBuf, sync::Arc};

use anyhow::Context as _;
use bytes::Bytes;
use futures::stream::BoxStream;

use self::{cask::PreparedCask, download::Download, formula::PreparedFormula};
use super::{PackageExt, resolved::ResolvedPackage};
use crate::context::Context;

#[expect(clippy::large_enum_variant)]
pub(crate) enum PreparedPackage<Dl = ()> {
    Formula(PreparedFormula<Dl>),
    Cask(PreparedCask<Dl>),
}

impl TryFrom<ResolvedPackage> for PreparedPackage {
    type Error = anyhow::Error;

    fn try_from(resolved_package: ResolvedPackage) -> Result<Self, Self::Error> {
        let this = match resolved_package {
            ResolvedPackage::Formula(resolved_formula) => {
                let resolved_formula = Arc::into_inner(resolved_formula)
                    .context("`Arc<ResolvedFormula>` still has multiple strong references")?;

                let prepared_formula = PreparedFormula::try_from(resolved_formula)?;

                Self::Formula(prepared_formula)
            },
            ResolvedPackage::Cask(resolved_cask) => {
                let resolved_cask = Arc::into_inner(resolved_cask)
                    .context("`Arc<ResolvedCask>` still has multiple strong references")?;

                let prepared_cask = PreparedCask::try_from(resolved_cask)?;

                Self::Cask(prepared_cask)
            },
        };

        Ok(this)
    }
}

impl PreparedPackage<()> {
    pub(crate) async fn with_download(
        self,
        context: &Context,
    ) -> anyhow::Result<(
        PreparedPackage<Download>,
        BoxStream<'static, anyhow::Result<Bytes>>,
    )> {
        match self {
            Self::Formula(formula) => {
                let (formula, stream) = formula.with_download(context).await?;

                let package = PreparedPackage::Formula(formula);

                Ok((package, stream))
            },
            Self::Cask(cask) => {
                let (cask, stream) = cask.with_download(context).await?;

                let package = PreparedPackage::Cask(cask);

                Ok((package, stream))
            },
        }
    }
}

impl<Dl> PackageExt for PreparedPackage<Dl>
where
    PreparedFormula<Dl>: PackageExt,
    PreparedCask<Dl>: PackageExt,
{
    fn id(&self) -> &str {
        match self {
            Self::Formula(formula) => formula.id(),
            Self::Cask(cask) => cask.id(),
        }
    }

    fn version(&self) -> &str {
        match self {
            Self::Formula(formula) => formula.version(),
            Self::Cask(cask) => cask.version(),
        }
    }
}

pub(crate) trait PreparedPackageExt: PackageExt {
    type Download;

    fn is_compatible(&self) -> bool;

    async fn is_installed(&self, context: &Context) -> anyhow::Result<bool>;

    async fn is_up_to_date(&self, context: &Context) -> anyhow::Result<bool>;

    fn download(&self) -> &Self::Download;

    fn pour_dir_path(&self, context: &Context) -> PathBuf;
}

impl<Dl> PreparedPackageExt for PreparedPackage<Dl>
where
    PreparedFormula<Dl>: PreparedPackageExt<Download = Dl>,
    PreparedCask<Dl>: PreparedPackageExt<Download = Dl>,
{
    type Download = Dl;

    fn is_compatible(&self) -> bool {
        match self {
            Self::Formula(formula) => formula.is_compatible(),
            Self::Cask(cask) => cask.is_compatible(),
        }
    }

    async fn is_installed(&self, context: &Context) -> anyhow::Result<bool> {
        match self {
            Self::Formula(formula) => formula.is_installed(context).await,
            Self::Cask(cask) => cask.is_installed(context).await,
        }
    }

    async fn is_up_to_date(&self, context: &Context) -> anyhow::Result<bool> {
        match self {
            Self::Formula(formula) => formula.is_up_to_date(context).await,
            Self::Cask(cask) => cask.is_up_to_date(context).await,
        }
    }

    fn download(&self) -> &Dl {
        match self {
            Self::Formula(formula) => formula.download(),
            Self::Cask(cask) => cask.download(),
        }
    }

    fn pour_dir_path(&self, context: &Context) -> PathBuf {
        match self {
            Self::Formula(formula) => formula.pour_dir_path(context),
            Self::Cask(cask) => cask.pour_dir_path(context),
        }
    }
}
