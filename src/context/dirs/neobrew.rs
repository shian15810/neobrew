#[cfg(debug_assertions)]
use std::path::PathBuf;

use anyhow::Result;
#[cfg(debug_assertions)]
use etcetera::BaseStrategy as _;
use etcetera::base_strategy;

type NeobrewBaseStrategy = cfg_select! {
    target_os = "windows" => base_strategy::Windows,
    any(target_os = "macos", target_os = "ios") => base_strategy::Xdg,
    _ => base_strategy::Xdg,
};

pub(crate) struct NeobrewDirs {
    strategy: NeobrewBaseStrategy,
}

impl NeobrewDirs {
    const APP_NAME: &str = "Neobrew";

    pub(in super::super) fn new() -> Result<Self> {
        let strategy = base_strategy::choose_base_strategy()?;

        let this = Self {
            strategy,
        };

        Ok(this)
    }

    #[cfg(debug_assertions)]
    pub(crate) fn cache_dir(&self) -> PathBuf {
        let cache_dir = self.strategy.cache_dir();

        cache_dir.join(Self::APP_NAME)
    }

    #[cfg(debug_assertions)]
    fn prefix_dir(&self) -> PathBuf {
        let app_name = Self::APP_NAME.to_lowercase();

        let dot_app_name = format!(".{app_name}");

        let home_dir = self.strategy.home_dir();

        home_dir.join(dot_app_name)
    }

    #[cfg(debug_assertions)]
    pub(crate) fn cellar_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("Cellar")
    }

    #[cfg(debug_assertions)]
    pub(crate) fn caskroom_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("Caskroom")
    }
}
