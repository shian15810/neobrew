mod cask;
mod formula;

use std::sync::Arc;

use self::{cask::CaskLinker, formula::FormulaLinker};
use crate::{
    context::Context,
    package::{pipelined::PipelinedPackage, prepared::PreparedPackage},
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

    pub(crate) async fn link(&self, pipelined_package: &PipelinedPackage) -> anyhow::Result<()> {
        match pipelined_package {
            PipelinedPackage::Formula(pipelined_formula) => {
                self.formula_linker.link(pipelined_formula).await?;
            },
            PipelinedPackage::Cask(pipelined_cask) => {
                self.cask_linker.link(pipelined_cask).await?;
            },
        }

        Ok(())
    }
}

trait Linkerer {
    type PreparedPackage;
    type PipelinedPackage;

    async fn is_up_to_date(&self, prepared_package: &Self::PreparedPackage)
    -> anyhow::Result<bool>;

    async fn is_installed(&self, prepared_package: &Self::PreparedPackage) -> anyhow::Result<bool>;

    async fn link(&self, pipelined_package: &Self::PipelinedPackage) -> anyhow::Result<()>;
}
