use std::{
    fs,
    path::Path,
    sync::{Arc, LazyLock},
};

use anyhow::Result;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

use crate::{
    context::Context,
    ext::std::path::PathExt as _,
    package::{Packageable as _, fetched::FetchedFormula},
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

        let handle = task::spawn_blocking(move || {
            for must_exist_dir_name in MUST_EXIST_DIR_NAMES.as_slice() {
                let must_exist_dir_path = prefix_dir_path.join(must_exist_dir_name);

                fs::create_dir_all(must_exist_dir_path)?;
            }

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        let this = Self {
            context,
        };

        Ok(this)
    }

    pub(crate) async fn link_opt(&self, fetched_formula: &FetchedFormula) -> Result<()> {
        let id = fetched_formula.id();

        let version = fetched_formula.version();

        let opt_prefix_symlink_path = self.context.homebrew_dirs.opt_prefix_symlink(id);

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        let handle = task::spawn_blocking(move || {
            keg_dir_path.create_relative_symlink_atomically_at(opt_prefix_symlink_path)?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }

    pub(crate) async fn link_keg(&self, fetched_formula: &FetchedFormula) -> Result<()> {
        let id = fetched_formula.id();

        let version = fetched_formula.version();

        let prefix_dir_path = self.context.homebrew_dirs.prefix_dir();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        let linked_keg_prefix_symlink_path =
            self.context.homebrew_dirs.linked_keg_prefix_symlink(id);

        let handle = task::spawn_blocking(move || {
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
                )?;
            }

            keg_dir_path.create_relative_symlink_atomically_at(linked_keg_prefix_symlink_path)?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }

    fn link_dir(src_dir_path: &Path, dest_dir_path: &Path, should_skip: bool) -> Result<()> {
        fs::create_dir_all(dest_dir_path)?;

        for entry in fs::read_dir(src_dir_path)? {
            let entry = entry?;

            let src_path = entry.path();

            let dest_path = dest_dir_path.join(entry.file_name());

            if src_path.is_dir() {
                if should_skip {
                    continue;
                }

                Self::link_dir(&src_path, &dest_path, false)?;

                continue;
            }

            src_path.create_relative_symlink_atomically_at(dest_path)?;
        }

        Ok(())
    }
}
