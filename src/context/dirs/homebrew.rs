use std::path::PathBuf;

use anyhow::Result;
use etcetera::{BaseStrategy as _, base_strategy};

use super::super::config::HomebrewEnvConfig;

type HomebrewBaseStrategy = cfg_select! {
    target_os = "windows" => base_strategy::Windows,
    any(target_os = "macos", target_os = "ios") => base_strategy::Apple,
    _ => base_strategy::Xdg,
};

pub(crate) struct HomebrewDirs {
    strategy: HomebrewBaseStrategy,
}

impl HomebrewDirs {
    const APP_NAME: &str = "Homebrew";

    pub(in super::super) fn new() -> Result<Self> {
        let strategy = base_strategy::choose_native_strategy()?;

        let this = Self {
            strategy,
        };

        Ok(this)
    }

    pub(crate) fn cache_dir(&self) -> PathBuf {
        let cache_dir = self.strategy.cache_dir();

        cache_dir.join(Self::APP_NAME)
    }

    fn prefix_dir() -> PathBuf {
        PathBuf::from(HomebrewEnvConfig::DEFAULT_PREFIX)
    }

    pub(crate) fn cellar_dir() -> PathBuf {
        let prefix_dir = Self::prefix_dir();

        prefix_dir.join("Cellar")
    }

    pub(crate) fn caskroom_dir() -> PathBuf {
        let prefix_dir = Self::prefix_dir();

        prefix_dir.join("Caskroom")
    }
}
