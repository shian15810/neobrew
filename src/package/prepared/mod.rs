mod cask;
mod formula;

use std::sync::Arc;

use anyhow::{Result, anyhow};
use enum_dispatch::enum_dispatch;

pub(crate) use self::{cask::PreparedCask, formula::PreparedFormula};
use super::{Packageable, resolved::ResolvedPackage};

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
