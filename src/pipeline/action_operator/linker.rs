use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use anyhow::anyhow;
use async_recursion::async_recursion;
use async_trait::async_trait;
use futures::future;
use tokio::fs;

use super::ActionOperator;
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    package::{
        Packageable as _,
        prepared::{PreparedFormula, PreparedPackage},
    },
    pipeline::state_store::{LinkedOutput, RelocatedOutput, Stage},
};

const KEG_LINK_DIR_NAMES: &[&str] = &["bin", "etc", "include", "lib", "sbin", "share", "var"];

static MUST_EXIST_SUBDIR_NAMES: LazyLock<Vec<&str>> = LazyLock::new(|| {
    KEG_LINK_DIR_NAMES
        .iter()
        .copied()
        .filter(|&keg_link_dir_name| keg_link_dir_name != "var")
        .chain(["opt", "var/homebrew/linked"])
        .collect::<Vec<_>>()
});

static MUST_EXIST_DIR_NAMES: LazyLock<Vec<&str>> = LazyLock::new(|| {
    MUST_EXIST_SUBDIR_NAMES
        .iter()
        .copied()
        .chain(["Cellar"])
        .collect::<Vec<_>>()
});

const SKIP_LINK_DIR_NAMES: &[&str] = &["bin", "sbin"];

pub(crate) struct Linker;

#[async_trait]
impl ActionOperator for Linker {
    type Input = RelocatedOutput;
    type Staging = (PathBuf, Option<PathBuf>);
    type Output = LinkedOutput;

    async fn should_run(
        &self,
        _input: Option<&Self::Input>,
        prepared_package: &PreparedPackage,
    ) -> anyhow::Result<bool> {
        let PreparedPackage::Formula(_prepared_formula) = prepared_package else {
            return Ok(false);
        };

        Ok(true)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        Some("Linking")
    }

    async fn execute(
        &self,
        _input: Option<&Self::Input>,
        prepared_package: &PreparedPackage,
        context: &Context,
    ) -> anyhow::Result<Self::Staging> {
        let PreparedPackage::Formula(prepared_formula) = prepared_package else {
            let err = anyhow!("`PreparedCask` is not supposed to be linked");

            return Err(err);
        };

        let (opt_prefix_link_path, linked_keg_prefix_link_path) =
            self.link(prepared_formula, context).await?;

        Ok((opt_prefix_link_path, linked_keg_prefix_link_path))
    }

    fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output> {
        let (opt_prefix_link_path, linked_keg_prefix_link_path) = staging;

        let output = LinkedOutput {
            opt_prefix_link_path,
            linked_keg_prefix_link_path,
        };

        Ok(output)
    }

    fn passed_stage(&self, _should_run: bool, prepared_package: &PreparedPackage) -> Option<Stage> {
        let PreparedPackage::Formula(_prepared_formula) = prepared_package else {
            return None;
        };

        Some(Stage::Linked)
    }
}

impl Linker {
    pub(crate) fn new() -> Self {
        Self
    }

    async fn link(
        &self,
        prepared_formula: &PreparedFormula,
        context: &Context,
    ) -> anyhow::Result<(PathBuf, Option<PathBuf>)> {
        let opt_prefix_link_path = self.link_opt(prepared_formula, context).await?;

        let linked_keg_prefix_link_path = if prepared_formula.should_link_keg() {
            let linked_keg_prefix_link_path = self.link_keg(prepared_formula, context).await?;

            Some(linked_keg_prefix_link_path)
        } else {
            None
        };

        Ok((opt_prefix_link_path, linked_keg_prefix_link_path))
    }

    async fn link_opt(
        &self,
        prepared_formula: &PreparedFormula,
        context: &Context,
    ) -> anyhow::Result<PathBuf> {
        let id = prepared_formula.id();

        let version_revision = prepared_formula.version_revision();

        let keg_dir_path = context.homebrew_dirs.keg_dir(id, version_revision);

        let opt_prefix_link_path = context.homebrew_dirs.opt_prefix_link(id);

        let opt_prefix_link_base_path = opt_prefix_link_path.base()?;

        fs::create_dir_all(opt_prefix_link_base_path).await?;

        keg_dir_path
            .create_relative_link_atomically_at(&opt_prefix_link_path)
            .await?;

        Ok(opt_prefix_link_path)
    }

    async fn link_keg(
        &self,
        prepared_formula: &PreparedFormula,
        context: &Context,
    ) -> anyhow::Result<PathBuf> {
        let id = prepared_formula.id();

        let version_revision = prepared_formula.version_revision();

        let keg_dir_path = context.homebrew_dirs.keg_dir(id, version_revision);

        let prefix_dir_path = context.homebrew_dirs.prefix_dir();

        let linked_keg_prefix_link_path = context.homebrew_dirs.linked_keg_prefix_link(id);

        let keg_link_dir_name_futs = KEG_LINK_DIR_NAMES.iter().map(async |keg_link_dir_name| {
            let keg_link_dir_path = keg_dir_path.join(keg_link_dir_name);

            if !keg_link_dir_path.try_exists()? {
                return Ok(());
            }

            let prefix_link_dir_path = prefix_dir_path.join(keg_link_dir_name);

            let should_skip = SKIP_LINK_DIR_NAMES.contains(keg_link_dir_name);

            self.link_dir(&keg_link_dir_path, &prefix_link_dir_path, should_skip)
                .await?;

            anyhow::Ok(())
        });

        future::try_join_all(keg_link_dir_name_futs).await?;

        let linked_keg_prefix_link_base_path = linked_keg_prefix_link_path.base()?;

        fs::create_dir_all(linked_keg_prefix_link_base_path).await?;

        keg_dir_path
            .create_relative_link_atomically_at(&linked_keg_prefix_link_path)
            .await?;

        Ok(linked_keg_prefix_link_path)
    }

    #[expect(clippy::self_only_used_in_recursion)]
    #[async_recursion]
    async fn link_dir(
        &self,
        src_base_path: &Path,
        dest_base_path: &Path,
        should_skip: bool,
    ) -> anyhow::Result<()> {
        let mut src_base_entries = fs::read_dir(src_base_path).await?;

        while let Some(src_base_entry) = src_base_entries.next_entry().await? {
            let src_file_name = src_base_entry.file_name();

            let src_file_path = src_base_entry.path();

            let dest_file_path = dest_base_path.join(src_file_name);

            if src_file_path.is_dir_exists_nofollow().await? {
                if should_skip {
                    continue;
                }

                self.link_dir(&src_file_path, &dest_file_path, false)
                    .await?;

                continue;
            }

            fs::create_dir_all(dest_base_path).await?;

            src_file_path
                .create_relative_link_atomically_at(dest_file_path)
                .await?;
        }

        Ok(())
    }

    async fn is_linked(
        &self,
        prepared_formula: &PreparedFormula,
        context: &Context,
    ) -> anyhow::Result<bool> {
        let is_opt_linked = self.is_opt_linked(prepared_formula, context).await?;

        if !is_opt_linked {
            return Ok(false);
        }

        let should_link_keg = prepared_formula.should_link_keg();

        if should_link_keg {
            let is_keg_linked = self.is_keg_linked(prepared_formula, context).await?;

            if !is_keg_linked {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn is_opt_linked(
        &self,
        prepared_formula: &PreparedFormula,
        context: &Context,
    ) -> anyhow::Result<bool> {
        let id = prepared_formula.id();

        let version_revision = prepared_formula.version_revision();

        let keg_dir_path = context.homebrew_dirs.keg_dir(id, version_revision);

        let opt_prefix_link_path = context.homebrew_dirs.opt_prefix_link(id);

        let is_opt_dir_exists = keg_dir_path.is_dir_exists_nofollow().await?;

        let is_opt_link_exists = opt_prefix_link_path.is_link_exists_nofollow().await?;

        let is_opt_link_valid = keg_dir_path.realpath_or_none().await?
            == opt_prefix_link_path.realpath_or_none().await?;

        let is_opt_linked = is_opt_dir_exists && is_opt_link_exists && is_opt_link_valid;

        Ok(is_opt_linked)
    }

    async fn is_keg_linked(
        &self,
        prepared_formula: &PreparedFormula,
        context: &Context,
    ) -> anyhow::Result<bool> {
        let id = prepared_formula.id();

        let version_revision = prepared_formula.version_revision();

        let keg_dir_path = context.homebrew_dirs.keg_dir(id, version_revision);

        let linked_keg_prefix_link_path = context.homebrew_dirs.linked_keg_prefix_link(id);

        let is_keg_dir_exists = keg_dir_path.is_dir_exists_nofollow().await?;

        let is_keg_link_exists = linked_keg_prefix_link_path
            .is_link_exists_nofollow()
            .await?;

        let is_keg_link_valid = keg_dir_path.realpath_or_none().await?
            == linked_keg_prefix_link_path.realpath_or_none().await?;

        let is_keg_linked = is_keg_dir_exists && is_keg_link_exists && is_keg_link_valid;

        Ok(is_keg_linked)
    }
}
