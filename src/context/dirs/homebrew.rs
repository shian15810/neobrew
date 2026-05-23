use std::path::PathBuf;

use anyhow::Result;
use etcetera::{BaseStrategy, base_strategy};

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

impl ProjectDirs for HomebrewDirs {
    #[cfg(all(debug_assertions, target_os = "linux"))]
    fn cache_dir(&self) -> PathBuf {
        let cache_dir = self.strategy().data_dir();

        cache_dir.join(Self::APP_NAME).join("cache")
    }

    #[cfg(not(all(debug_assertions, target_os = "linux")))]
    fn cache_dir(&self) -> PathBuf {
        let cache_dir = self.strategy().cache_dir();

        cache_dir.join(Self::APP_NAME)
    }
}

impl HomebrewDirs {
    #[cfg(debug_assertions)]
    pub(crate) fn prefix_dir(&self) -> PathBuf {
        let app_name = Self::APP_NAME.to_lowercase();

        let dot_app_name = format!(".{app_name}");

        let home_dir = self.strategy.home_dir();

        home_dir.join(dot_app_name)
    }

    #[cfg(not(debug_assertions))]
    pub(crate) fn prefix_dir(&self) -> PathBuf {
        use super::super::config::HomebrewEnvConfig;

        PathBuf::from(HomebrewEnvConfig::DEFAULT_PREFIX)
    }

    pub(crate) fn opt_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("opt")
    }

    pub(crate) fn opt_prefix_symlink(&self, id: &str) -> PathBuf {
        let opt_dir = self.opt_dir();

        opt_dir.join(id)
    }

    pub(crate) fn cellar_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("Cellar")
    }

    pub(crate) fn rack_dir(&self, id: &str) -> PathBuf {
        let cellar_dir = self.cellar_dir();

        cellar_dir.join(id)
    }

    pub(crate) fn keg_dir(&self, id: &str, version: &str) -> PathBuf {
        let rack_dir = self.rack_dir(id);

        rack_dir.join(version)
    }

    pub(crate) fn linked_keg_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("var/homebrew/linked")
    }

    pub(crate) fn linked_keg_prefix_symlink(&self, id: &str) -> PathBuf {
        let linked_keg_dir = self.linked_keg_dir();

        linked_keg_dir.join(id)
    }

    pub(crate) fn caskroom_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("Caskroom")
    }

    pub(crate) fn repository_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        cfg_select! {
            all(target_os = "macos", target_arch = "aarch64") => prefix_dir,
            all(target_os = "macos", target_arch = "x86_64") => prefix_dir.join("Homebrew"),
            target_os = "linux" => prefix_dir.join("Homebrew"),
        }
    }

    pub(crate) fn library_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("Library")
    }
}
