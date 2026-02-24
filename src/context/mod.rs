use std::{num::NonZeroUsize, sync::LazyLock, thread};

use anyhow::Result;
use etcetera::app_strategy;
use once_cell::sync::OnceCell as OnceLock;

use self::{
    configs::{Config, HomebrewConfig, NeobrewConfig},
    project_dirs::ProjectDirs,
};

mod configs;
mod project_dirs;

static CONCURRENCY_LIMIT: LazyLock<usize> = LazyLock::new(|| {
    thread::available_parallelism()
        .unwrap_or(NonZeroUsize::MIN)
        .get()
        .min(Context::MAX_CONCURRENCY)
});

pub struct Context {
    project_dirs: OnceLock<ProjectDirs>,

    neobrew_config: OnceLock<NeobrewConfig>,
    homebrew_config: OnceLock<HomebrewConfig>,

    pub client: LazyLock<reqwest::Client>,

    pub concurrency_limit: LazyLock<usize>,
    pub channel_capacity: LazyLock<usize>,
}

impl Context {
    const MAX_CONCURRENCY: usize = 1 << 4;
    const BUFFER_MULTIPLIER: usize = 1 << 4;

    pub fn new() -> Self {
        Self {
            project_dirs: OnceLock::new(),

            neobrew_config: OnceLock::new(),
            homebrew_config: OnceLock::new(),

            client: LazyLock::new(reqwest::Client::new),

            concurrency_limit: LazyLock::new(|| *CONCURRENCY_LIMIT),
            channel_capacity: LazyLock::new(|| *CONCURRENCY_LIMIT * Self::BUFFER_MULTIPLIER),
        }
    }

    pub fn project_dirs(&self) -> Result<&app_strategy::Xdg> {
        let project_dirs = self.project_dirs.get_or_try_init(ProjectDirs::new)?;

        let strategy = project_dirs.strategy();

        Ok(strategy)
    }

    fn neobrew_config(&self) -> Result<&NeobrewConfig> {
        self.neobrew_config.get_or_try_init(NeobrewConfig::load)
    }

    pub fn homebrew_config(&self) -> Result<&HomebrewConfig> {
        self.homebrew_config.get_or_try_init(HomebrewConfig::load)
    }
}
