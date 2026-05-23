mod cask;
mod formula;

use std::{
    fs::File,
    io::{self, ErrorKind},
    path::Path,
    sync::Arc,
};

use anyhow::{Result, anyhow};
use base16ct::HexDisplay;
use bytes::Bytes;
use digest_io::IoWrapper;
use enum_dispatch::enum_dispatch;
use futures::stream::{Stream, StreamExt as _, TryStreamExt as _};
use oci_client::secrets::RegistryAuth;
use sha2::{Digest as _, Sha256};
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

pub(crate) use self::{cask::PreparedCask, formula::PreparedFormula};
use super::{Packageable, resolved::ResolvedPackage};
use crate::{
    context::Context,
    pipeline::{pull_operator::TempPourerInput, push_operator::TempWriterInput},
};

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

#[enum_dispatch(PreparedPackage)]
pub(crate) trait PreparedPackageable: Packageable {
    fn expected_sha256(&self) -> &str;

    async fn temp_writer_input(&self, context: &Context) -> Result<TempWriterInput>;
}

impl PreparedPackage {
    pub(crate) async fn cache_file_sha256(&self, file_path: &Path) -> Result<Option<String>> {
        let file_path = file_path.to_owned();

        let handle = task::spawn_blocking(move || {
            let mut file = match File::open(file_path) {
                Ok(file) => file,
                Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
                Err(err) => return Err(err)?,
            };

            let mut hasher = IoWrapper(Sha256::new());

            io::copy(&mut file, &mut hasher)?;

            anyhow::Ok(Some(hasher))
        });
        let handle = AbortOnDropHandle::new(handle);

        let Some(hasher) = handle.await?? else {
            return Ok(None);
        };

        let file_sha256 = hasher.0.finalize();
        let file_sha256 = HexDisplay(&file_sha256);
        let file_sha256 = format!("{file_sha256:x}");

        Ok(Some(file_sha256))
    }

    pub(crate) fn temp_pourer_input(&self, context: &Context) -> TempPourerInput {
        let dir_path = match self {
            Self::Formula(_) => context.homebrew_dirs.cellar_dir(),
            Self::Cask(_) => context.homebrew_dirs.caskroom_dir(),
        };

        TempPourerInput::new(dir_path)
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
