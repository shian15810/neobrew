#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use std::{cmp::Reverse, path::Path, sync::Arc};

use async_walkdir::WalkDir;
use futures::stream::StreamExt as _;

#[cfg(target_os = "linux")]
pub(crate) use self::linux::Relocation;
#[cfg(target_os = "macos")]
pub(crate) use self::macos::Relocation;
use crate::{
    context::Context,
    ext::tokio::path::PathExt as _,
    package::{Packageable as _, streamed::StreamedFormula},
};

#[expect(private_bounds)]
pub(crate) trait Relocator: RelocatorInner {
    fn new(context: Arc<Context>) -> Self {
        let homebrew_dirs = &context.homebrew_dirs;

        let replacement_pairs = [
            (Self::PREFIX_PLACEHOLDER, homebrew_dirs.prefix_dir()),
            (Self::CELLAR_PLACEHOLDER, homebrew_dirs.cellar_dir()),
            (Self::REPOSITORY_PLACEHOLDER, homebrew_dirs.repository_dir()),
            (Self::LIBRARY_PLACEHOLDER, homebrew_dirs.library_dir()),
        ];
        let mut replacement_pairs = replacement_pairs.map(|(placeholder, replacement_path)| {
            let replacement_pstr = replacement_path.to_string_lossy();
            let replacement_pstr = replacement_pstr.into_owned();

            (placeholder, replacement_pstr)
        });

        replacement_pairs.sort_by_key(|(placeholder, _)| Reverse(placeholder.len()));

        Self::from((replacement_pairs, context))
    }

    async fn patch(&self, streamed_formula: &StreamedFormula) -> anyhow::Result<()> {
        let cellar_dir_path = self.context().homebrew_dirs.cellar_dir();

        if streamed_formula.should_relocate(&cellar_dir_path) {
            let keg_dir_path = self
                .context()
                .homebrew_dirs
                .keg_dir(streamed_formula.id(), streamed_formula.version());

            self.patch_keg(&keg_dir_path).await?;
        }

        Ok(())
    }
}

trait RelocatorInner: From<([(&'static str, String); 4], Arc<Context>)> {
    const PREFIX_PLACEHOLDER: &str = "@@HOMEBREW_PREFIX@@";
    const CELLAR_PLACEHOLDER: &str = "@@HOMEBREW_CELLAR@@";
    const REPOSITORY_PLACEHOLDER: &str = "@@HOMEBREW_REPOSITORY@@";
    const LIBRARY_PLACEHOLDER: &str = "@@HOMEBREW_LIBRARY@@";

    fn replacement_pairs(&self) -> &[(&'static str, String); 4];

    fn context(&self) -> &Context;

    async fn patch_keg(&self, keg_dir_path: &Path) -> anyhow::Result<()> {
        let mut entries = WalkDir::new(keg_dir_path);

        while let Some(entry) = entries.next().await {
            let path = entry?.path();

            if !path.is_file_exists_nofollow().await? {
                continue;
            }

            self.patch_file(&path).await?;
        }

        Ok(())
    }

    async fn patch_file(&self, path: &Path) -> anyhow::Result<()>;

    fn replace_bytes(&self, bytes: &[u8]) -> anyhow::Result<Vec<u8>>;

    fn replace_text(&self, text: &str) -> String {
        self.replacement_pairs()
            .iter()
            .fold(text.to_owned(), |text, (placeholder, replacement)| {
                text.replace(placeholder, replacement)
            })
    }
}
