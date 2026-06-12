mod homebrew;
mod neobrew;

use std::path::PathBuf;

use etcetera::BaseStrategy;

pub(crate) use self::homebrew::HomebrewDirs;
pub(super) use self::neobrew::NeobrewDirs;

trait ProjectDirsInner {
    const APP_NAME: &str;

    fn strategy(&self) -> &impl BaseStrategy;
}

#[expect(private_bounds)]
pub(crate) trait ProjectDirs: ProjectDirsInner {
    fn home_dir(&self) -> PathBuf {
        self.strategy().home_dir().to_owned()
    }

    fn data_dir(&self) -> PathBuf {
        let data_dir = self.strategy().data_dir();

        let app_name = cfg_select! {
            target_os = "macos" => cfg_select! {
                debug_assertions => Self::APP_NAME.to_lowercase(),
                not(debug_assertions) => Self::APP_NAME,
            },
            target_os = "linux" => Self::APP_NAME.to_lowercase(),
        };

        data_dir.join(app_name)
    }

    fn cache_dir(&self) -> PathBuf {
        let cache_dir = self.strategy().cache_dir();

        let app_name = cfg_select! {
            target_os = "macos" => cfg_select! {
                debug_assertions => Self::APP_NAME.to_lowercase(),
                not(debug_assertions) => Self::APP_NAME,
            },
            target_os = "linux" => Self::APP_NAME.to_lowercase(),
        };

        cache_dir.join(app_name)
    }
}
