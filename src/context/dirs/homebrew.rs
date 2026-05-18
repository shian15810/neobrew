use std::path::PathBuf;

use anyhow::Result;
use etcetera::{BaseStrategy, base_strategy};

use super::{super::config::HomebrewEnvConfig, ProjectDirs, ProjectDirsInner};

type HomebrewBaseStrategy = cfg_select! {
    target_os = "windows" => base_strategy::Windows,
    any(target_os = "macos", target_os = "ios") => base_strategy::Apple,
    _ => base_strategy::Xdg,
};

pub(crate) struct HomebrewDirs {
    strategy: HomebrewBaseStrategy,
}

impl HomebrewDirs {
    pub(in super::super) fn new() -> Result<Self> {
        let strategy = base_strategy::choose_native_strategy()?;

        let this = Self {
            strategy,
        };

        Ok(this)
    }
}

impl ProjectDirsInner for HomebrewDirs {
    const APP_NAME: &str = "Homebrew";

    fn strategy(&self) -> &impl BaseStrategy {
        &self.strategy
    }

    fn prefix_dir(&self) -> PathBuf {
        PathBuf::from(HomebrewEnvConfig::DEFAULT_PREFIX)
    }
}

impl ProjectDirs for HomebrewDirs {}
