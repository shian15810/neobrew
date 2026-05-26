use std::{
    path::Path,
    sync::{Arc, LazyLock},
};

use anyhow::Result;
use async_recursion::async_recursion;
use tokio::fs;

use crate::{
    context::Context,
    ext::tokio::path::PathExt as _,
    package::{Packageable as _, fetched::FetchedFormula, prepared::PreparedFormula},
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

pub(crate) struct Linker {
    context: Arc<Context>,
}

impl Linker {
    pub(crate) async fn create(context: Arc<Context>) -> Result<Self> {
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

    pub(crate) async fn link_opt(&self, fetched_formula: &FetchedFormula) -> Result<()> {
        let id = fetched_formula.id();

        let version = fetched_formula.version();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        let opt_prefix_symlink_path = self.context.homebrew_dirs.opt_prefix_symlink(id);

        keg_dir_path
            .create_relative_symlink_atomically_at(opt_prefix_symlink_path)
            .await?;

        Ok(())
    }

    pub(crate) async fn link_keg(&self, fetched_formula: &FetchedFormula) -> Result<()> {
        let id = fetched_formula.id();

        let version = fetched_formula.version();

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

            let should_skip_link_dir = SKIP_LINK_DIR_NAMES.contains(keg_link_dir_name);

            Self::link_dir(
                &keg_link_dir_path,
                &prefix_link_dir_path,
                should_skip_link_dir,
            )
            .await?;
        }

        keg_dir_path
            .create_relative_symlink_atomically_at(linked_keg_prefix_symlink_path)
            .await?;

        Ok(())
    }

    #[async_recursion]
    async fn link_dir(src_dir_path: &Path, dest_dir_path: &Path, should_skip: bool) -> Result<()> {
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

    async fn is_up_to_date(&self, prepared_formula: &PreparedFormula) -> Result<bool> {
        let id = prepared_formula.id();

        let version = prepared_formula.version();

        let keg_dir = self.context.homebrew_dirs.keg_dir(id, version);

        if keg_dir.is_dir_exists_nofollow().await? {
            return Ok(true);
        }

        Ok(false)
    }

    async fn is_installed(&self, prepared_formula: &PreparedFormula) -> Result<bool> {
        let id = prepared_formula.id();

        let rack_dir = self.context.homebrew_dirs.rack_dir(id);

        if !rack_dir.is_dir_exists_nofollow().await? {
            return Ok(false);
        }

        let mut rack_dir_entries = fs::read_dir(rack_dir).await?;

        while let Some(rack_dir_entry) = rack_dir_entries.next_entry().await? {
            if rack_dir_entry.path().is_dir_exists_nofollow().await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn is_linked(&self, prepared_formula: &PreparedFormula) -> Result<bool> {
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

    async fn is_opt_linked(&self, prepared_formula: &PreparedFormula) -> Result<bool> {
        let id = prepared_formula.id();

        let version = prepared_formula.version();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        let opt_prefix_symlink_path = self.context.homebrew_dirs.opt_prefix_symlink(id);

        let is_opt_linked = keg_dir_path.is_dir_exists_nofollow().await?
            && opt_prefix_symlink_path.is_symlink_exists_nofollow().await?
            && keg_dir_path.canonicalize()? == opt_prefix_symlink_path.canonicalize()?;

        Ok(is_opt_linked)
    }

    async fn is_keg_linked(&self, prepared_formula: &PreparedFormula) -> Result<bool> {
        let id = prepared_formula.id();

        let version = prepared_formula.version();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        let linked_keg_prefix_symlink_path =
            self.context.homebrew_dirs.linked_keg_prefix_symlink(id);

        let is_keg_linked = keg_dir_path.is_dir_exists_nofollow().await?
            && linked_keg_prefix_symlink_path
                .is_symlink_exists_nofollow()
                .await?
            && keg_dir_path.canonicalize()? == linked_keg_prefix_symlink_path.canonicalize()?;

        Ok(is_keg_linked)
    }
}
