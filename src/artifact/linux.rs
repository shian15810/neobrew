use std::sync::Arc;

use super::Artifactable;
use crate::{context::Context, package::prepared::PreparedCask, placeholder::Placeholder};

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
    async fn relocate(&self, _prepared_cask: &PreparedCask) -> anyhow::Result<()> {
        Ok(())
    }
}
