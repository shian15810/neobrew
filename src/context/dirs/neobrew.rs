use std::path::PathBuf;

use etcetera::{BaseStrategy, base_strategy};

use super::{ProjectDirs, ProjectDirsInner};

type NeobrewBaseStrategy = cfg_select! {
    target_os = "windows" => base_strategy::Windows,
    target_os = "macos" => base_strategy::Xdg,
    target_os = "ios" => base_strategy::Xdg,
    _ => base_strategy::Xdg,
};

pub(crate) struct NeobrewDirs {
    strategy: NeobrewBaseStrategy,
}

impl ProjectDirsInner for NeobrewDirs {
    const APP_NAME: &str = "Neobrew";

    fn strategy(&self) -> &impl BaseStrategy {
        &self.strategy
    }
}

impl NeobrewDirs {
    pub(in super::super) fn load() -> anyhow::Result<Self> {
        let strategy = base_strategy::choose_base_strategy()?;

        let this = Self {
            strategy,
        };

        Ok(this)
    }
}

impl ProjectDirs for NeobrewDirs {}

impl NeobrewDirs {
    fn config_dir(&self) -> PathBuf {
        let config_dir = self.strategy.config_dir();

        let app_name = Self::APP_NAME.to_lowercase();

        config_dir.join(app_name)
    }
}
