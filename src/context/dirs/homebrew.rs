use std::path::PathBuf;

use etcetera::{BaseStrategy, base_strategy};

use super::{ProjectDirs, ProjectDirsInner};

#[cfg(debug_assertions)]
type HomebrewBaseStrategy = cfg_select! {
    target_os = "windows" => base_strategy::Windows,
    target_os = "macos" => base_strategy::Xdg,
    target_os = "ios" => base_strategy::Xdg,
    _ => base_strategy::Xdg,
};

#[cfg(not(debug_assertions))]
type HomebrewNativeStrategy = cfg_select! {
    target_os = "windows" => base_strategy::Windows,
    target_os = "macos" => base_strategy::Apple,
    target_os = "ios" => base_strategy::Apple,
    _ => base_strategy::Xdg,
};

pub(crate) struct HomebrewDirs {
    #[cfg(debug_assertions)]
    strategy: HomebrewBaseStrategy,

    #[cfg(not(debug_assertions))]
    strategy: HomebrewNativeStrategy,
}

impl ProjectDirsInner for HomebrewDirs {
    const APP_NAME: &str = "Homebrew";

    fn strategy(&self) -> &impl BaseStrategy {
        &self.strategy
    }
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

impl ProjectDirs for HomebrewDirs {
    #[cfg(debug_assertions)]
    fn home_dir(&self) -> PathBuf {
        let data_dir = self.data_dir();

        let app_name = Self::APP_NAME.to_lowercase();

        data_dir.join("home").join(app_name)
    }

    #[cfg(all(target_os = "linux", debug_assertions))]
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

        let dot_app_name = format!(".{app_name}");

        home_dir.join(dot_app_name)
    }

    #[cfg(not(debug_assertions))]
    pub(crate) fn prefix_dir(&self) -> PathBuf {
        use super::super::config::homebrew_env::HomebrewEnvConfig;

        PathBuf::from(HomebrewEnvConfig::DEFAULT_PREFIX)
    }

    fn opt_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("opt")
    }

    pub(crate) fn opt_prefix_link(&self, id: &str) -> PathBuf {
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

    fn linked_keg_dir(&self) -> PathBuf {
        let prefix_dir = self.prefix_dir();

        prefix_dir.join("var/homebrew/linked")
    }

    pub(crate) fn linked_keg_prefix_link(&self, id: &str) -> PathBuf {
        let linked_keg_dir = self.linked_keg_dir();

        linked_keg_dir.join(id)
    }

    fn caskroom_dir(&self) -> PathBuf {
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
            target_os = "macos" => cfg_select! {
                target_arch = "aarch64" => prefix_dir,
                target_arch = "x86_64" => prefix_dir.join("Homebrew"),
            },
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

    #[expect(clippy::unused_self)]
    fn root_dir(&self) -> PathBuf {
        PathBuf::from("/")
    }

    #[cfg(all(target_os = "macos", debug_assertions))]
    pub(crate) fn install_dir(&self) -> PathBuf {
        self.data_dir()
    }

    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    pub(crate) fn install_dir(&self) -> PathBuf {
        self.root_dir()
    }

    #[cfg(all(target_os = "macos", debug_assertions))]
    pub(crate) fn app_dir(&self) -> PathBuf {
        let data_dir = self.data_dir();

        data_dir.join("Applications")
    }

    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    pub(crate) fn app_dir(&self) -> PathBuf {
        let root_dir = self.root_dir();

        root_dir.join("Applications")
    }

    #[cfg(target_os = "macos")]
    fn lib_dir(&self) -> PathBuf {
        let home_dir = self.home_dir();

        home_dir.join("Library")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn colorpicker_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("ColorPickers")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn dictionary_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Dictionaries")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn font_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Fonts")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn input_method_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Input Methods")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn internet_plugin_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Internet Plug-Ins")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn keyboard_layout_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Keyboard Layouts")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn prefpane_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("PreferencePanes")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn qlplugin_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("QuickLook")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn mdimporter_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Spotlight")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn screen_saver_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Screen Savers")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn service_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Services")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn audio_unit_plugin_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Audio/Plug-Ins/Components")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn vst_plugin_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Audio/Plug-Ins/VST")
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn vst3_plugin_dir(&self) -> PathBuf {
        let lib_dir = self.lib_dir();

        lib_dir.join("Audio/Plug-Ins/VST3")
    }
}
