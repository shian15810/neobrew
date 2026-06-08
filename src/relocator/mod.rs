#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use std::{borrow::Cow, cmp::Reverse, path::Path, sync::Arc};

use async_walkdir::WalkDir;
use futures::stream::StreamExt as _;

#[cfg(target_os = "linux")]
pub(crate) use self::linux::Relocator;
#[cfg(target_os = "macos")]
pub(crate) use self::macos::Relocator;
use crate::{
    context::Context,
    ext::tokio::path::PathExt as _,
    package::{Packageable as _, prepared::PreparedFormula},
};

#[expect(private_bounds)]
pub(crate) trait Relocate: RelocateInner {
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

    async fn patch(&self, prepared_formula: &PreparedFormula) -> anyhow::Result<()> {
        let id = prepared_formula.id();

        let version_revision = prepared_formula.version_revision();

        let cellar_dir_path = self.context().homebrew_dirs.cellar_dir();

        if prepared_formula.should_relocate(&cellar_dir_path) {
            let keg_dir_path = self.context().homebrew_dirs.keg_dir(id, version_revision);

            self.patch_keg(&keg_dir_path).await?;
        }

        Ok(())
    }
}

trait RelocateInner: From<([(&'static str, String); 4], Arc<Context>)> {
    const PREFIX_PLACEHOLDER: &str = "@@HOMEBREW_PREFIX@@";
    const CELLAR_PLACEHOLDER: &str = "@@HOMEBREW_CELLAR@@";
    const REPOSITORY_PLACEHOLDER: &str = "@@HOMEBREW_REPOSITORY@@";
    const LIBRARY_PLACEHOLDER: &str = "@@HOMEBREW_LIBRARY@@";

    fn replacement_pairs(&self) -> &[(&'static str, String); 4];

    fn context(&self) -> &Context;

    async fn patch_keg(&self, keg_dir_path: &Path) -> anyhow::Result<()> {
        let mut keg_entries = WalkDir::new(keg_dir_path);

        while let Some(keg_entry) = keg_entries.next().await {
            let keg_entry = keg_entry?;

            let keg_entry_path = keg_entry.path();

            if !keg_entry_path.is_file_exists_nofollow().await? {
                continue;
            }

            self.patch_file(&keg_entry_path).await?;
        }

        Ok(())
    }

    async fn patch_file(&self, dest_file_path: &Path) -> anyhow::Result<()>;

    fn replace_bytes(&self, bytes: &[u8]) -> anyhow::Result<Vec<u8>>;

    fn replace_pstr<'a>(&self, text: &'a str) -> Cow<'a, str> {
        self.replacement_pairs().iter().fold(
            Cow::Borrowed(text),
            |current, (placeholder, replacement_pstr)| {
                if current.contains(placeholder) {
                    let text = current.replace(placeholder, replacement_pstr);

                    Cow::Owned(text)
                } else {
                    current
                }
            },
        )
    }
}
