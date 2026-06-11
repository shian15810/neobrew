use std::path::PathBuf;

use super::{Artifactor, Artifactory};
use crate::package::prepared::PreparedCask;

impl Artifactory for Artifactor {
    #[expect(clippy::unused_async_trait_impl)]
    async fn relocate(
        &self,
        prepared_cask: &PreparedCask,
        context: &Context,
    ) -> anyhow::Result<PathBuf> {
        let id = prepared_cask.id();

        let version = prepared_cask.version();

        let staged_dir_path = context.homebrew_dirs.staged_dir(id, version);

        Ok(staged_dir_path)
    }

    #[expect(clippy::unused_async_trait_impl)]
    async fn link(
        &self,
        prepared_cask: &PreparedCask,
        context: &Context,
    ) -> anyhow::Result<PathBuf> {
        let id = prepared_cask.id();

        let version = prepared_cask.version();

        let staged_dir_path = context.homebrew_dirs.staged_dir(id, version);

        Ok(staged_dir_path)
    }
}
