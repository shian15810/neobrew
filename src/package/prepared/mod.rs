mod cask;
mod cask_stanza;
mod formula;

use std::sync::Arc;

use anyhow::Context as _;
use enum_dispatch::enum_dispatch;

pub(crate) use self::{
    cask::PreparedCask,
    cask_stanza::{CommonStanza, Stanzas},
    formula::PreparedFormula,
};
use super::{Packageable, resolved::ResolvedPackage};
use crate::context::Context;

#[expect(clippy::large_enum_variant)]
#[enum_dispatch]
pub(crate) enum PreparedPackage {
    Formula(PreparedFormula),
    Cask(PreparedCask),
}

impl TryFrom<(ResolvedPackage, bool)> for PreparedPackage {
    type Error = Option<anyhow::Error>;

    fn try_from(
        (resolved_package, is_requested): (ResolvedPackage, bool),
    ) -> Result<Self, Self::Error> {
        let this = match resolved_package {
            ResolvedPackage::Formula(resolved_formula) => {
                let resolved_formula = Arc::into_inner(resolved_formula)
                    .context("`Arc<ResolvedFormula>` still has multiple strong references")?;

                let prepared_formula = PreparedFormula::try_from((resolved_formula, is_requested))?;

                Self::Formula(prepared_formula)
            },
            ResolvedPackage::Cask(resolved_cask) => {
                let resolved_cask = Arc::into_inner(resolved_cask)
                    .context("`Arc<ResolvedCask>` still has multiple strong references")?;

                let prepared_cask = PreparedCask::try_from((resolved_cask, is_requested))?;

                Self::Cask(prepared_cask)
            },
        };

        Ok(this)
    }
}

#[enum_dispatch(PreparedPackage)]
pub(crate) trait PreparedPackageable: Packageable {
    fn download_url(&self) -> &str;

    fn expected_sha256(&self) -> &str;

    async fn is_installed(&self, context: &Context) -> anyhow::Result<bool>;

    async fn is_up_to_date(&self, context: &Context) -> anyhow::Result<bool>;
}
