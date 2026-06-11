#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use std::{
    borrow::Cow,
    cmp::Reverse,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use async_trait::async_trait;
use async_walkdir::WalkDir;
use bytes::Bytes;
use futures::stream::StreamExt as _;

use super::{
    super::state_store::{PouredOutput, RelocatedOutput, Stage},
    SensorOperator,
};
use crate::{
    context::Context,
    ext::tokio::path::PathExt as _,
    package::{
        Packageable as _,
        prepared::{PreparedFormula, PreparedPackage},
    },
};

#[derive(Clone)]
pub(crate) struct Relocator {
    replacement_pairs: [(&'static str, String); 4],
}

#[async_trait]
impl SensorOperator for Relocator {
    type Payload = PouredOutput;
    type Staging = PathBuf;
    type Output = RelocatedOutput;

    fn should_run(&self, prepared_package: &PreparedPackage, context: &Context) -> bool {
        let PreparedPackage::Formula(prepared_formula) = prepared_package else {
            return false;
        };

        let cellar_dir_path = context.homebrew_dirs.cellar_dir();

        prepared_formula.should_relocate(&cellar_dir_path)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        Some("Relocating")
    }

    fn poke_stage(&self) -> Stage {
        Stage::Poured
    }

    async fn execute(
        &self,
        _payload: &Self::Payload,
        prepared_package: &PreparedPackage,
        context: &Context,
    ) -> anyhow::Result<Self::Staging> {
        let PreparedPackage::Formula(prepared_formula) = prepared_package else {
            let err = anyhow!("`PreparedCask` is not supposed to be relocated");

            return Err(err);
        };

        let keg_dir_path = self.patch(prepared_formula, context).await?;

        Ok(keg_dir_path)
    }

    fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output> {
        let keg_dir_path = staging;

        let relocated_output = RelocatedOutput {
            keg_dir_path,
        };

        Ok(relocated_output)
    }

    fn passed_stage(&self, _should_run: bool, prepared_package: &PreparedPackage) -> Option<Stage> {
        let PreparedPackage::Formula(_prepared_formula) = prepared_package else {
            return None;
        };

        Some(Stage::Relocated)
    }
}

impl Relocator {
    const PREFIX_PLACEHOLDER: &str = "@@HOMEBREW_PREFIX@@";
    const CELLAR_PLACEHOLDER: &str = "@@HOMEBREW_CELLAR@@";
    const REPOSITORY_PLACEHOLDER: &str = "@@HOMEBREW_REPOSITORY@@";
    const LIBRARY_PLACEHOLDER: &str = "@@HOMEBREW_LIBRARY@@";

    pub(crate) fn new(context: &Context) -> Self {
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

        Self {
            replacement_pairs,
        }
    }

    async fn patch(
        &self,
        prepared_formula: &PreparedFormula,
        context: &Context,
    ) -> anyhow::Result<PathBuf> {
        let id = prepared_formula.id();

        let version_revision = prepared_formula.version_revision();

        let keg_dir_path = context.homebrew_dirs.keg_dir(id, version_revision);

        self.patch_keg(&keg_dir_path).await?;

        Ok(keg_dir_path)
    }

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

    fn replace_pstr<'a>(&self, text: &'a str) -> Cow<'a, str> {
        self.replacement_pairs.iter().fold(
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

trait Relocatory {
    async fn patch_file(&self, dest_file_path: &Path) -> anyhow::Result<()>;

    fn replace_bytes(&self, bytes: &Bytes) -> anyhow::Result<Vec<u8>>;
}
