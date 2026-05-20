use std::{borrow::Cow, fs::File, io, iter, path::PathBuf, sync::Arc};

use anyhow::{Context as _, Result, anyhow};
use base16ct::HexDisplay;
use bytes::Bytes;
use digest_io::IoWrapper;
use either::Either::{Left, Right};
use enum_dispatch::enum_dispatch;
use futures::stream::{Stream, StreamExt as _, TryStreamExt as _};
use oci_client::secrets::RegistryAuth;
use pathdiff::diff_paths;
use sha2::{Digest as _, Sha256};
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

use self::{
    cask::{FetchedCask, PreparedCask},
    formula::{FetchedFormula, PreparedFormula},
};
pub(crate) use self::{
    cask::{RawCask, ResolvedCask},
    formula::{RawFormula, ResolvedFormula},
};
use crate::context::Context;

mod cask;
mod formula;

#[enum_dispatch]
enum Package {
    Raw(RawPackage),
    Resolved(ResolvedPackage),
    Prepared(PreparedPackage),
    Fetched(FetchedPackage),
}

#[enum_dispatch(Package, RawPackage, ResolvedPackage, PreparedPackage, FetchedPackage)]
pub(crate) trait Packageable {
    fn id(&self) -> &str;

    fn version(&self) -> &str;
}

impl<Package: Packageable> Packageable for Arc<Package> {
    fn id(&self) -> &str {
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        this.id()
    }

    fn version(&self) -> &str {
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        this.version()
    }
}

#[enum_dispatch]
pub(crate) enum RawPackage {
    Formula(RawFormula),
    Cask(RawCask),
}

#[expect(shadowing_supertrait_items)]
#[enum_dispatch(RawPackage)]
pub(crate) trait RawPackageable: Packageable {
    fn version(&self) -> Cow<'_, str>;

    fn cache(&self, context: &Context) -> RawPackageCache;
}

pub(crate) struct RawPackageCache {
    pub(crate) file_location_parent: PathBuf,
    pub(crate) file_location: PathBuf,
}

#[enum_dispatch]
pub(crate) enum ResolvedPackage {
    Formula(Arc<ResolvedFormula>),
    Cask(Arc<ResolvedCask>),
}

impl ResolvedPackage {
    pub(crate) fn iter(&self) -> impl Iterator<Item = Self> + use<> {
        match self {
            Self::Formula(formula) => {
                let formulae = formula.iter().map(Self::Formula);

                Left(formulae)
            },

            Self::Cask(cask) => {
                let cask = Arc::clone(cask);

                let casks = iter::once(cask).map(Self::Cask);

                Right(casks)
            },
        }
    }
}

#[expect(shadowing_supertrait_items)]
#[enum_dispatch(ResolvedPackage)]
trait ResolvedPackageable: Packageable {
    fn version(&self) -> Cow<'_, str>;
}

impl<ResolvedPackage: ResolvedPackageable> ResolvedPackageable for Arc<ResolvedPackage> {
    fn version(&self) -> Cow<'_, str> {
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        #[expect(resolving_to_items_shadowing_supertrait_items)]
        this.version()
    }
}

#[enum_dispatch]
pub(crate) enum PreparedPackage {
    Formula(PreparedFormula),
    Cask(PreparedCask),
}

impl TryFrom<ResolvedPackage> for PreparedPackage {
    type Error = Option<anyhow::Error>;

    fn try_from(resolved_package: ResolvedPackage) -> Result<Self, Self::Error> {
        let this = match resolved_package {
            ResolvedPackage::Formula(resolved_formula) => {
                let Some(resolved_formula) = Arc::into_inner(resolved_formula) else {
                    let err =
                        anyhow!("`Arc<ResolvedFormula>` still has multiple strong references");

                    return Err(Some(err));
                };

                let prepared_formula = PreparedFormula::try_from(resolved_formula)?;

                Self::Formula(prepared_formula)
            },

            ResolvedPackage::Cask(resolved_cask) => {
                let Some(resolved_cask) = Arc::into_inner(resolved_cask) else {
                    let err = anyhow!("`Arc<ResolvedCask>` still has multiple strong references");

                    return Err(Some(err));
                };

                let prepared_cask = PreparedCask::from(resolved_cask);

                Self::Cask(prepared_cask)
            },
        };

        Ok(this)
    }
}

impl PreparedPackage {
    pub(crate) fn dest(&self, context: &Context) -> PreparedPackageDest {
        let id = self.id();

        let version = self.version();

        let dest_dir = match self {
            Self::Formula(_) => context.homebrew_dirs.cellar_dir(),
            Self::Cask(_) => context.homebrew_dirs.caskroom_dir(),
        };

        let dir_location_grandparent = dest_dir;

        let dir_location_parent = dir_location_grandparent.join(id);

        let dir_location = dir_location_parent.join(version);

        PreparedPackageDest {
            id: id.to_owned(),
            version: version.to_owned(),

            dir_location_grandparent,
            dir_location_parent,
            dir_location,
        }
    }

    pub(crate) async fn stream(
        &self,
        context: &Context,
    ) -> Result<Option<impl Stream<Item = Result<Bytes>> + Send + 'static>> {
        let stream = match self {
            Self::Formula(prepared_formula) => {
                let Some(oci) = prepared_formula.oci() else {
                    return Ok(None);
                };

                context
                    .oci_client
                    .store_auth_if_needed(oci.registry, &RegistryAuth::Anonymous)
                    .await;

                let stream = context
                    .oci_client
                    .pull_blob_stream(&oci.reference, &oci.descriptor)
                    .await?;
                let stream = stream.err_into::<anyhow::Error>();

                stream.left_stream()
            },

            Self::Cask(prepared_cask) => {
                let url = prepared_cask.url();

                let resp = context.client.get(url).send().await?;
                let resp = resp.error_for_status()?;

                let stream = resp.bytes_stream();
                let stream = stream.err_into::<anyhow::Error>();

                stream.right_stream()
            },
        };

        Ok(Some(stream))
    }
}

#[expect(private_bounds)]
#[enum_dispatch(PreparedPackage)]
pub(crate) trait PreparedPackageable: PreparedPackageableInner {
    async fn cache(&self, context: &Context) -> Result<PreparedPackageCache>;

    fn sha256(&self) -> &str;
}

#[enum_dispatch(PreparedPackage)]
trait PreparedPackageableInner: Packageable {
    fn cache_inner(
        &self,
        file_name: &str,
        symlink_name: &str,
        symlink_location_parent: PathBuf,
    ) -> PreparedPackageCache {
        let file_location_parent = symlink_location_parent.join("downloads");

        let file_location = file_location_parent.join(file_name);

        let symlink_location = symlink_location_parent.join(symlink_name);

        PreparedPackageCache {
            file_location_parent,
            file_location,

            symlink_location_parent,
            symlink_location,
        }
    }
}

#[derive(Clone)]
pub(crate) struct PreparedPackageDest {
    pub(crate) id: String,
    pub(crate) version: String,

    pub(crate) dir_location_grandparent: PathBuf,
    pub(crate) dir_location_parent: PathBuf,
    pub(crate) dir_location: PathBuf,
}

pub(crate) struct PreparedPackageCache {
    pub(crate) file_location_parent: PathBuf,
    pub(crate) file_location: PathBuf,

    pub(crate) symlink_location_parent: PathBuf,
    pub(crate) symlink_location: PathBuf,
}

impl PreparedPackageCache {
    pub(crate) async fn file_sha256(&self) -> Result<String> {
        let file_location = self.file_location.clone();

        let handle = task::spawn_blocking(move || {
            let mut file = File::open(file_location)?;

            let mut hasher = IoWrapper(Sha256::new());

            io::copy(&mut file, &mut hasher)?;

            let file_sha256 = hasher.0.finalize();
            let file_sha256 = HexDisplay(&file_sha256);
            let file_sha256 = format!("{file_sha256:x}");

            anyhow::Ok(file_sha256)
        });
        let handle = AbortOnDropHandle::new(handle);

        let file_sha256 = handle.await??;

        Ok(file_sha256)
    }

    pub(crate) fn symlink_location_diff(&self) -> Result<PathBuf> {
        let symlink_location_diff = diff_paths(&self.file_location, &self.symlink_location_parent)
            .context("Failed to diff paths")?;

        Ok(symlink_location_diff)
    }

    pub(crate) fn symlink_location_tmp(&self) -> PathBuf {
        self.symlink_location.with_extension("tmp")
    }
}

#[enum_dispatch]
pub(crate) enum FetchedPackage {
    Formula(FetchedFormula),
    Cask(FetchedCask),
}

impl From<(PreparedPackage, PreparedPackageDest)> for FetchedPackage {
    fn from((prepared_package, dest): (PreparedPackage, PreparedPackageDest)) -> Self {
        match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                let fetched_formula = FetchedFormula::from((prepared_formula, dest));

                Self::Formula(fetched_formula)
            },

            PreparedPackage::Cask(prepared_cask) => {
                let fetched_cask = FetchedCask::from((prepared_cask, dest));

                Self::Cask(fetched_cask)
            },
        }
    }
}
