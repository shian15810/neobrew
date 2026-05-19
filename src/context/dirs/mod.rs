mod homebrew;
mod neobrew;

use std::path::PathBuf;

use etcetera::BaseStrategy;

pub(super) use self::{homebrew::HomebrewDirs, neobrew::NeobrewDirs};

trait ProjectDirsInner {
    const APP_NAME: &str;

    fn strategy(&self) -> &impl BaseStrategy;
}

#[expect(private_bounds)]
pub(crate) trait ProjectDirs: ProjectDirsInner {
    fn cache_dir(&self) -> PathBuf {
        let cache_dir = self.strategy().cache_dir();

        cache_dir.join(Self::APP_NAME)
    }
}
