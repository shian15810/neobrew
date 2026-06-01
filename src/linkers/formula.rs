use std::{
    path::Path,
    sync::{Arc, LazyLock},
};

use async_recursion::async_recursion;
use tokio::fs;

use super::Linkerer;
use crate::{
    context::Context,
    ext::tokio::path::PathExt as _,
    package::{Packageable as _, prepared::PreparedFormula, streamed::StreamedFormula},
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

pub(super) struct FormulaLinker {
    context: Arc<Context>,
}

impl Linkerer for FormulaLinker {
    type PreparedPackage = PreparedFormula;
    type StreamedPackage = StreamedFormula;

    async fn is_updated(&self, prepared_package: &PreparedFormula) -> anyhow::Result<bool> {
        let prepared_formula = prepared_package;

        if !self.is_installed(prepared_formula).await? {
            return Ok(false);
        }

        let id = prepared_formula.id();

        let version = prepared_formula.version();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        if keg_dir_path.is_dir_exists_nofollow().await? {
            return Ok(true);
        }

        Ok(false)
    }

    async fn is_installed(&self, prepared_package: &PreparedFormula) -> anyhow::Result<bool> {
        let prepared_formula = prepared_package;

        let id = prepared_formula.id();

        let rack_dir_path = self.context.homebrew_dirs.rack_dir(id);

        if !rack_dir_path.is_dir_exists_nofollow().await? {
            return Ok(false);
        }

        let mut rack_dir_entries = fs::read_dir(rack_dir_path).await?;

        while let Some(rack_dir_entry) = rack_dir_entries.next_entry().await? {
            if rack_dir_entry.path().is_dir_exists_nofollow().await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn link(&self, streamed_package: &StreamedFormula) -> anyhow::Result<()> {
        let streamed_formula = streamed_package;

        self.link_opt(streamed_formula).await?;

        if streamed_formula.should_link_keg() {
            self.link_keg(streamed_formula).await?;
        }

        Ok(())
    }
}

impl FormulaLinker {
    pub(super) async fn try_init(context: Arc<Context>) -> anyhow::Result<Self> {
        let prefix_dir_path = context.homebrew_dirs.prefix_dir();

        for must_exist_dir_name in MUST_EXIST_DIR_NAMES.as_slice() {
            let must_exist_dir_path = prefix_dir_path.join(must_exist_dir_name);

            fs::create_dir_all(must_exist_dir_path).await?;
        }

        let this = Self {
            context,
        };

        Ok(this)
    }

    async fn link_opt(&self, streamed_formula: &StreamedFormula) -> anyhow::Result<()> {
        let id = streamed_formula.id();

        let version = streamed_formula.version();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        let opt_prefix_symlink_path = self.context.homebrew_dirs.opt_prefix_symlink(id);

        keg_dir_path
            .create_relative_symlink_atomically_at(opt_prefix_symlink_path)
            .await?;

        Ok(())
    }

    async fn link_keg(&self, streamed_formula: &StreamedFormula) -> anyhow::Result<()> {
        let id = streamed_formula.id();

        let version = streamed_formula.version();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        let prefix_dir_path = self.context.homebrew_dirs.prefix_dir();

        let linked_keg_prefix_symlink_path =
            self.context.homebrew_dirs.linked_keg_prefix_symlink(id);

        for keg_link_dir_name in KEG_LINK_DIR_NAMES {
            let keg_link_dir_path = keg_dir_path.join(keg_link_dir_name);

            if !keg_link_dir_path.try_exists()? {
                continue;
            }

            let prefix_link_dir_path = prefix_dir_path.join(keg_link_dir_name);

            let should_skip = SKIP_LINK_DIR_NAMES.contains(keg_link_dir_name);

            Self::link_dir(&keg_link_dir_path, &prefix_link_dir_path, should_skip).await?;
        }

        keg_dir_path
            .create_relative_symlink_atomically_at(linked_keg_prefix_symlink_path)
            .await?;

        Ok(())
    }

    #[async_recursion]
    async fn link_dir(
        src_dir_path: &Path,
        dest_dir_path: &Path,
        should_skip: bool,
    ) -> anyhow::Result<()> {
        fs::create_dir_all(dest_dir_path).await?;

        let mut src_dir_entries = fs::read_dir(src_dir_path).await?;

        while let Some(src_dir_entry) = src_dir_entries.next_entry().await? {
            let src_path = src_dir_entry.path();

            let dest_path = dest_dir_path.join(src_dir_entry.file_name());

            if src_path.is_dir_exists_nofollow().await? {
                if should_skip {
                    continue;
                }

                Self::link_dir(&src_path, &dest_path, false).await?;

                continue;
            }

            src_path
                .create_relative_symlink_atomically_at(dest_path)
                .await?;
        }

        Ok(())
    }

    async fn is_linked(&self, prepared_formula: &PreparedFormula) -> anyhow::Result<bool> {
        let is_opt_linked = self.is_opt_linked(prepared_formula).await?;

        if !is_opt_linked {
            return Ok(false);
        }

        let should_link_keg = prepared_formula.should_link_keg();

        if should_link_keg {
            let is_keg_linked = self.is_keg_linked(prepared_formula).await?;

            if !is_keg_linked {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn is_opt_linked(&self, prepared_formula: &PreparedFormula) -> anyhow::Result<bool> {
        let id = prepared_formula.id();

        let version = prepared_formula.version();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        let opt_prefix_symlink_path = self.context.homebrew_dirs.opt_prefix_symlink(id);

        let is_opt_dir_exists = keg_dir_path.is_dir_exists_nofollow().await?;

        let is_opt_symlink_exists = opt_prefix_symlink_path.is_symlink_exists_nofollow().await?;

        let is_opt_symlink_valid = keg_dir_path.realpath_or_none().await?
            == opt_prefix_symlink_path.realpath_or_none().await?;

        let is_opt_linked = is_opt_dir_exists && is_opt_symlink_exists && is_opt_symlink_valid;

        Ok(is_opt_linked)
    }

    async fn is_keg_linked(&self, prepared_formula: &PreparedFormula) -> anyhow::Result<bool> {
        let id = prepared_formula.id();

        let version = prepared_formula.version();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        let linked_keg_prefix_symlink_path =
            self.context.homebrew_dirs.linked_keg_prefix_symlink(id);

        let is_keg_dir_exists = keg_dir_path.is_dir_exists_nofollow().await?;

        let is_keg_symlink_exists = linked_keg_prefix_symlink_path
            .is_symlink_exists_nofollow()
            .await?;

        let is_keg_symlink_valid = keg_dir_path.realpath_or_none().await?
            == linked_keg_prefix_symlink_path.realpath_or_none().await?;

        let is_keg_linked = is_keg_dir_exists && is_keg_symlink_exists && is_keg_symlink_valid;

        Ok(is_keg_linked)
    }
}
