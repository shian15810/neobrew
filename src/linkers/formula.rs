use std::{
    path::Path,
    sync::{Arc, LazyLock},
};

use async_recursion::async_recursion;
use futures::future;
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

    async fn is_installed(&self, prepared_package: &PreparedFormula) -> anyhow::Result<bool> {
        let prepared_formula = prepared_package;

        let id = prepared_formula.id();

        let rack_dir_path = self.context.homebrew_dirs.rack_dir(id);

        if !rack_dir_path.is_dir_exists_nofollow().await? {
            return Ok(false);
        }

        let mut rack_dir_entries = fs::read_dir(rack_dir_path).await?;

        while let Some(rack_dir_entry) = rack_dir_entries.next_entry().await? {
            let rack_dir_entry_path = rack_dir_entry.path();

            let is_rack_dir_entry_exists = rack_dir_entry_path.is_dir_exists_nofollow().await?;

            let is_rack_dir_entry_not_empty = !rack_dir_entry_path.is_dir_empty().await?;

            if is_rack_dir_entry_exists && is_rack_dir_entry_not_empty {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn is_up_to_date(&self, prepared_package: &PreparedFormula) -> anyhow::Result<bool> {
        let prepared_formula = prepared_package;

        let id = prepared_formula.id();

        let version_revision = prepared_formula.version_revision();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version_revision);

        let is_keg_dir_exists = keg_dir_path.is_dir_exists_nofollow().await?;

        let is_keg_dir_not_empty = !keg_dir_path.is_dir_empty().await?;

        if is_keg_dir_exists && is_keg_dir_not_empty {
            return Ok(true);
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

        let must_exist_dir_name_futs =
            MUST_EXIST_DIR_NAMES
                .iter()
                .map(async |must_exist_dir_name| {
                    let must_exist_dir_path = prefix_dir_path.join(must_exist_dir_name);

                    fs::create_dir_all(must_exist_dir_path).await?;

                    anyhow::Ok(())
                });

        future::try_join_all(must_exist_dir_name_futs).await?;

        let this = Self {
            context,
        };

        Ok(this)
    }

    async fn link_opt(&self, streamed_formula: &StreamedFormula) -> anyhow::Result<()> {
        let id = streamed_formula.id();

        let version_revision = streamed_formula.version_revision();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version_revision);

        let opt_prefix_symlink_path = self.context.homebrew_dirs.opt_prefix_symlink(id);

        keg_dir_path
            .create_relative_symlink_atomically_at(opt_prefix_symlink_path)
            .await?;

        Ok(())
    }

    async fn link_keg(&self, streamed_formula: &StreamedFormula) -> anyhow::Result<()> {
        let id = streamed_formula.id();

        let version_revision = streamed_formula.version_revision();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version_revision);

        let prefix_dir_path = self.context.homebrew_dirs.prefix_dir();

        let linked_keg_prefix_symlink_path =
            self.context.homebrew_dirs.linked_keg_prefix_symlink(id);

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

        keg_dir_path
            .create_relative_symlink_atomically_at(linked_keg_prefix_symlink_path)
            .await?;

        Ok(())
    }

    #[expect(clippy::self_only_used_in_recursion)]
    #[async_recursion]
    async fn link_dir(
        &self,
        src_base_path: &Path,
        dest_base_path: &Path,
        should_skip: bool,
    ) -> anyhow::Result<()> {
        fs::create_dir_all(dest_base_path).await?;

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

            src_file_path
                .create_relative_symlink_atomically_at(dest_file_path)
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

        let version_revision = prepared_formula.version_revision();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version_revision);

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

        let version_revision = prepared_formula.version_revision();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version_revision);

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
