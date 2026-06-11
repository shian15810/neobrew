#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use std::path::PathBuf;

use anyhow::anyhow;
use async_trait::async_trait;
use path_clean::PathClean as _;

use super::{
    super::state_store::{ArtifactedOutput, PouredOutput, Stage},
    SensorOperator,
};
#[cfg(target_os = "macos")]
use crate::context::dirs::ProjectDirs as _;
use crate::{
    context::Context,
    package::prepared::{PreparedCask, PreparedPackage},
};

pub(crate) struct Artifactor {
    #[cfg(target_os = "macos")]
    replacement_pairs: [(&'static str, String); 4],

    #[cfg(target_os = "linux")]
    replacement_pairs: [(&'static str, String); 3],
}

#[async_trait]
impl SensorOperator for Artifactor {
    type Payload = PouredOutput;
    type Staging = PathBuf;
    type Output = ArtifactedOutput;

    fn should_run(&self, prepared_package: &PreparedPackage, _context: &Context) -> bool {
        let PreparedPackage::Cask(_prepared_cask) = prepared_package else {
            return false;
        };

        true
    }

    fn running_prefix(&self) -> Option<&'static str> {
        Some("Installing")
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
        let PreparedPackage::Cask(prepared_cask) = prepared_package else {
            let err = anyhow!("`PreparedFormula` is not supposed to be artifacted");

            return Err(err);
        };

        let _staged_dir_path = self.relocate(prepared_cask, context).await?;

        let staged_dir_path = self.link(prepared_cask, context).await?;

        Ok(staged_dir_path)
    }

    fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output> {
        let staged_dir_path = staging;

        let output = ArtifactedOutput {
            staged_dir_path,
        };

        Ok(output)
    }

    fn passed_stage(&self, _should_run: bool, prepared_package: &PreparedPackage) -> Option<Stage> {
        let PreparedPackage::Cask(_prepared_cask) = prepared_package else {
            return None;
        };

        Some(Stage::Artifacted)
    }
}

impl Artifactor {
    pub(crate) fn new(context: &Context) -> Self {
        let homebrew_dirs = &context.homebrew_dirs;

        #[cfg(target_os = "macos")]
        let replacement_pairs = [
            ("/$HOME", homebrew_dirs.home_dir()),
            ("$HOMEBREW_PREFIX", homebrew_dirs.prefix_dir()),
            ("$HOMEBREW_CELLAR", homebrew_dirs.cellar_dir()),
            ("$APPDIR", homebrew_dirs.app_dir()),
        ];

        #[cfg(target_os = "linux")]
        let replacement_pairs = [
            ("/$HOME", homebrew_dirs.home_dir()),
            ("$HOMEBREW_PREFIX", homebrew_dirs.prefix_dir()),
            ("$HOMEBREW_CELLAR", homebrew_dirs.cellar_dir()),
        ];

        let replacement_pairs = replacement_pairs.map(|(placeholder, replacement_path)| {
            let replacement_pstr = replacement_path.to_string_lossy();
            let replacement_pstr = replacement_pstr.into_owned();

            (placeholder, replacement_pstr)
        });

        Self {
            replacement_pairs,
        }
    }

    fn resolve_source(&self, pstr: &str) -> PathBuf {
        self.replace_pstr(pstr)
    }

    #[cfg(debug_assertions)]
    fn resolve_target(&self, pstr: &str, context: &Context) -> PathBuf {
        let path = self.replace_pstr(pstr);

        if path.is_relative() {
            return path;
        }

        let data_dir_path = context.homebrew_dirs.data_dir();

        let prefix_dir_path = context.homebrew_dirs.prefix_dir();

        if path.starts_with(&data_dir_path) || path.starts_with(prefix_dir_path) {
            return path;
        }

        match path.strip_prefix("/") {
            Ok(suffix_path) => data_dir_path.join(suffix_path),
            Err(_) => data_dir_path.join(path),
        }
    }

    #[cfg(not(debug_assertions))]
    fn resolve_target(&self, pstr: &str) -> PathBuf {
        self.replace_pstr(pstr)
    }

    fn replace_pstr(&self, pstr: &str) -> PathBuf {
        let pstr = match pstr.strip_prefix("~/") {
            Some(suffix_pstr) => format!("/$HOME/{suffix_pstr}"),
            None if pstr == "~" => "/$HOME".to_owned(),
            None => pstr.to_owned(),
        };

        #[cfg(target_os = "macos")]
        let pstr = match pstr.strip_prefix("/Applications/") {
            Some(suffix_pstr) => format!("$APPDIR/{suffix_pstr}"),
            None if pstr == "/Applications" => "$APPDIR".to_owned(),
            None => pstr,
        };

        let pstr = self
            .replacement_pairs
            .iter()
            .fold(pstr, |pstr, (placeholder, replacement_pstr)| {
                pstr.replace(placeholder, replacement_pstr)
            });

        let path = PathBuf::from(pstr);

        path.clean()
    }
}

trait Artifactory {
    async fn relocate(
        &self,
        prepared_cask: &PreparedCask,
        context: &Context,
    ) -> anyhow::Result<PathBuf>;

    async fn link(
        &self,
        prepared_cask: &PreparedCask,
        context: &Context,
    ) -> anyhow::Result<PathBuf>;
}
