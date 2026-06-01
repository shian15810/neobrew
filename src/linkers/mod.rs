mod cask;
mod formula;

use std::sync::Arc;

use self::{cask::CaskLinker, formula::FormulaLinker};
use crate::{
    context::Context,
    package::{prepared::PreparedPackage, streamed::StreamedPackage},
    placeholder::Placeholder,
};

pub(crate) struct Linkers {
    formula_linker: FormulaLinker,
    cask_linker: CaskLinker,

    context: Arc<Context>,
}

impl Linkers {
    pub(crate) async fn try_init(
        placeholder: Arc<Placeholder>,
        context: Arc<Context>,
    ) -> anyhow::Result<Self> {
        let this = Self {
            formula_linker: FormulaLinker::try_init(Arc::clone(&context)).await?,
            cask_linker: CaskLinker::try_init(placeholder, Arc::clone(&context)).await?,

            context,
        };

        Ok(this)
    }

    pub(crate) async fn is_installed(
        &self,
        prepared_package: &PreparedPackage,
    ) -> anyhow::Result<bool> {
        let is_installed = match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                self.formula_linker.is_installed(prepared_formula).await?
            },
            PreparedPackage::Cask(prepared_cask) => {
                self.cask_linker.is_installed(prepared_cask).await?
            },
        };

        Ok(is_installed)
    }

    pub(crate) async fn is_up_to_date(
        &self,
        prepared_package: &PreparedPackage,
    ) -> anyhow::Result<bool> {
        let is_up_to_date = match prepared_package {
            PreparedPackage::Formula(prepared_formula) => {
                self.formula_linker.is_up_to_date(prepared_formula).await?
            },
            PreparedPackage::Cask(prepared_cask) => {
                self.cask_linker.is_up_to_date(prepared_cask).await?
            },
        };

        Ok(is_up_to_date)
    }

    pub(crate) async fn link(&self, streamed_package: &StreamedPackage) -> anyhow::Result<()> {
        match streamed_package {
            StreamedPackage::Formula(streamed_formula) => {
                self.formula_linker.link(streamed_formula).await?;
            },
            StreamedPackage::Cask(streamed_cask) => {
                self.cask_linker.link(streamed_cask).await?;
            },
        }

        Ok(())
    }
}

trait Linkerer: Sized {
    type PreparedPackage;
    type StreamedPackage;

    async fn is_up_to_date(&self, prepared_package: &Self::PreparedPackage)
    -> anyhow::Result<bool>;

    async fn is_installed(&self, prepared_package: &Self::PreparedPackage) -> anyhow::Result<bool>;

    async fn link(&self, streamed_package: &Self::StreamedPackage) -> anyhow::Result<()>;
}
