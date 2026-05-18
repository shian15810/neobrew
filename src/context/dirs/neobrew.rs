use std::path::PathBuf;

use anyhow::Result;
use etcetera::{BaseStrategy, base_strategy};

use super::{ProjectDirs, ProjectDirsInner};

type NeobrewBaseStrategy = cfg_select! {
    target_os = "windows" => base_strategy::Windows,
    any(target_os = "macos", target_os = "ios") => base_strategy::Xdg,
    _ => base_strategy::Xdg,
};

pub(crate) struct NeobrewDirs {
    strategy: NeobrewBaseStrategy,
}

impl NeobrewDirs {
    pub(in super::super) fn new() -> Result<Self> {
        let strategy = base_strategy::choose_base_strategy()?;

        let this = Self {
            strategy,
        };

        Ok(this)
    }
}

impl ProjectDirsInner for NeobrewDirs {
    const APP_NAME: &str = "Neobrew";

    #[cfg(debug_assertions)]
    fn strategy(&self) -> &impl BaseStrategy {
        &self.strategy
    }

    #[cfg(not(debug_assertions))]
    fn strategy(&self) -> &impl BaseStrategy {
        unimplemented!();
    }

    #[cfg(debug_assertions)]
    fn prefix_dir(&self) -> PathBuf {
        let app_name = Self::APP_NAME.to_lowercase();

        let dot_app_name = format!(".{app_name}");

        let home_dir = self.strategy.home_dir();

        home_dir.join(dot_app_name)
    }

    #[cfg(not(debug_assertions))]
    fn prefix_dir(&self) -> PathBuf {
        unimplemented!();
    }
}

#[cfg(debug_assertions)]
impl ProjectDirs for NeobrewDirs {}

#[cfg(not(debug_assertions))]
impl ProjectDirs for NeobrewDirs {
    fn cache_dir(&self) -> PathBuf {
        unimplemented!();
    }

    fn cellar_dir(&self) -> PathBuf {
        unimplemented!();
    }

    fn caskroom_dir(&self) -> PathBuf {
        unimplemented!();
    }
}
