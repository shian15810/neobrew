use std::path::Path;

use super::{Artifactor, ArtifactorExt, ReplacementPairs};
use crate::{context::Context, package::prepared::cask_stanza::Stanzas};

impl ArtifactorExt for Artifactor {
    #[expect(clippy::unused_async_trait_impl)]
    async fn install(
        &self,
        _stanzas: &Stanzas,
        _staged_dir_path: &Path,
        _replacement_pairs: &ReplacementPairs,
        _context: &Context,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    #[expect(clippy::unused_async_trait_impl)]
    async fn relocate(
        &self,
        _stanzas: &Stanzas,
        _staged_dir_path: &Path,
        _replacement_pairs: &ReplacementPairs,
        _context: &Context,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    #[expect(clippy::unused_async_trait_impl)]
    async fn link(
        &self,
        _stanzas: &Stanzas,
        _staged_dir_path: &Path,
        _replacement_pairs: &ReplacementPairs,
        _context: &Context,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
