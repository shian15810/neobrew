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
    pub(in super::super) fn load() -> anyhow::Result<Self> {
        let strategy = base_strategy::choose_base_strategy()?;

        let this = Self {
            strategy,
        };

        Ok(this)
    }
}

impl ProjectDirsInner for NeobrewDirs {
    const APP_NAME: &str = "Neobrew";

    fn strategy(&self) -> &impl BaseStrategy {
        &self.strategy
    }
}

impl ProjectDirs for NeobrewDirs {}
