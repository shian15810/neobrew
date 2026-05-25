mod cask;
mod formula;

use std::sync::Arc;

use anyhow::{Result, anyhow};
use bytes::Bytes;
use enum_dispatch::enum_dispatch;
use futures::stream::{Stream, StreamExt as _, TryStreamExt as _};
use oci_client::secrets::RegistryAuth;

pub(crate) use self::{cask::PreparedCask, formula::PreparedFormula};
use super::{Packageable, resolved::ResolvedPackage};
use crate::context::Context;

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
    fn cache_url(&self) -> &str;

    fn expected_sha256(&self) -> &str;
}

impl PreparedPackage {
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
