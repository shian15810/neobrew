use std::sync::Arc;

use super::Artifactable;
use crate::{context::Context, package::pipelined::PipelinedCask, placeholder::Placeholder};

pub(crate) struct Artifact {
    placeholder: Arc<Placeholder>,

    context: Arc<Context>,
}

impl Artifactable for Artifact {
    fn new(placeholder: Arc<Placeholder>, context: Arc<Context>) -> Self {
        Self {
            placeholder,

            context,
        }
    }

    #[expect(clippy::unused_async_trait_impl)]
    async fn relocate(&self, _pipelined_cask: &PipelinedCask) -> anyhow::Result<()> {
        Ok(())
    }
}
