use std::path::PathBuf;

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
    pub(in super::super) fn load() -> anyhow::Result<Self> {
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
    #[cfg(debug_assertions)]
    fn home_dir(&self) -> PathBuf {
        let data_dir = self.data_dir();

        data_dir.join("home/brew")
    }

    #[cfg(all(debug_assertions, target_os = "linux"))]
    fn cache_dir(&self) -> PathBuf {
        let home_dir = self.home_dir();

        home_dir.join(".cache")
    }
}

impl HomebrewDirs {
    #[cfg(debug_assertions)]
    pub(crate) fn prefix_dir(&self) -> PathBuf {
        let home_dir = self.strategy.home_dir();

        let app_name = Self::APP_NAME.to_lowercase();

        home_dir.join(format!(".{app_name}"))
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

    pub(crate) fn cask_dir(&self, id: &str) -> PathBuf {
        let caskroom_dir = self.caskroom_dir();

        caskroom_dir.join(id)
    }

    pub(crate) fn staged_dir(&self, id: &str, version: &str) -> PathBuf {
        let cask_dir = self.cask_dir(id);

        cask_dir.join(version)
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
        let repository_dir = self.repository_dir();

        repository_dir.join("Library")
    }

    pub(crate) fn bin_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("bin")
    }

    pub(crate) fn man_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("share/man")
    }

    pub(crate) fn bash_completion_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("etc/bash_completion.d")
    }

    pub(crate) fn fish_completion_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("share/fish/vendor_completions.d")
    }

    pub(crate) fn zsh_completion_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("share/zsh/site-functions")
    }

    #[cfg(debug_assertions)]
    pub(crate) fn app_dir(&self) -> PathBuf {
        let data_dir = self.data_dir();

        data_dir.join("Applications")
    }

    #[cfg(not(debug_assertions))]
    pub(crate) fn app_dir(&self) -> PathBuf {
        PathBuf::from("/Applications")
    }

    pub(crate) fn lib_dir(&self) -> PathBuf {
        let home_dir = self.home_dir();

        home_dir.join("Library")
    }

    pub(crate) fn colorpicker_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("ColorPickers")
    }

    pub(crate) fn dictionary_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Dictionaries")
    }

    pub(crate) fn font_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Fonts")
    }

    pub(crate) fn input_method_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Input Methods")
    }

    pub(crate) fn internet_plugin_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Internet Plug-Ins")
    }

    pub(crate) fn keyboard_layout_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Keyboard Layouts")
    }

    pub(crate) fn prefpane_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("PreferencePanes")
    }

    pub(crate) fn mdimporter_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Spotlight")
    }

    pub(crate) fn screen_saver_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Screen Savers")
    }

    pub(crate) fn service_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Services")
    }

    pub(crate) fn audio_unit_plugin_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Audio/Plug-Ins/Components")
    }

    pub(crate) fn vst_plugin_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Audio/Plug-Ins/VST")
    }

    pub(crate) fn vst3_plugin_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Audio/Plug-Ins/VST3")
    }
}
