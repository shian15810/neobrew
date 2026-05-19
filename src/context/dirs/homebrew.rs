use std::path::PathBuf;

use anyhow::Result;
use etcetera::{BaseStrategy, base_strategy};

#[cfg(not(debug_assertions))]
use super::super::config::HomebrewEnvConfig;
use super::{ProjectDirs, ProjectDirsInner};

#[cfg(debug_assertions)]
type HomebrewBaseStrategy = cfg_select! {
    target_os = "windows" => base_strategy::Windows,
    any(target_os = "macos", target_os = "ios") => base_strategy::Xdg,
    _ => base_strategy::Xdg,
};

#[cfg(not(debug_assertions))]
type HomebrewNativeStrategy = cfg_select! {
    target_os = "windows" => base_strategy::Windows,
    any(target_os = "macos", target_os = "ios") => base_strategy::Apple,
    _ => base_strategy::Xdg,
};

pub(crate) struct HomebrewDirs {
    #[cfg(debug_assertions)]
    strategy: HomebrewBaseStrategy,

    #[cfg(not(debug_assertions))]
    strategy: HomebrewNativeStrategy,
}

impl HomebrewDirs {
    pub(in super::super) fn new() -> Result<Self> {
        #[cfg(debug_assertions)]
        let strategy = base_strategy::choose_base_strategy()?;

        #[cfg(not(debug_assertions))]
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
}

impl ProjectDirs for HomebrewDirs {}

impl HomebrewDirs {
    #[cfg(debug_assertions)]
    fn prefix_dir(&self) -> PathBuf {
        let app_name = Self::APP_NAME.to_lowercase();

        let dot_app_name = format!(".{app_name}");

        let home_dir = self.strategy.home_dir();

        home_dir.join(dot_app_name)
    }

    #[cfg(not(debug_assertions))]
    fn prefix_dir(&self) -> PathBuf {
        PathBuf::from(HomebrewEnvConfig::DEFAULT_PREFIX)
    }

    pub(crate) fn cellar_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("Cellar")
    }

    pub(crate) fn caskroom_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("Caskroom")
    }
}
