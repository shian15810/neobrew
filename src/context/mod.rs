use std::{num::NonZeroUsize, thread};

use anyhow::Result;
use etcetera::app_strategy;
use once_cell::sync::OnceCell as OnceLock;

use self::{
    configs::{Config, HomebrewConfig, NeobrewConfig},
    project_dirs::ProjectDirs,
};

mod configs;
mod project_dirs;

pub struct Context {
    project_dirs: OnceLock<ProjectDirs>,

    neobrew_config: OnceLock<NeobrewConfig>,
    homebrew_config: OnceLock<HomebrewConfig>,

    client: OnceLock<reqwest::Client>,

    concurrency_limit: OnceLock<usize>,
    channel_capacity: OnceLock<usize>,
}

impl Context {
    const MAX_CONCURRENCY: usize = 1 << 4;
    const BUFFER_MULTIPLIER: usize = 1 << 4;

    pub fn new() -> Self {
        Self {
            project_dirs: OnceLock::new(),

            neobrew_config: OnceLock::new(),
            homebrew_config: OnceLock::new(),

            client: OnceLock::new(),

            concurrency_limit: OnceLock::new(),
            channel_capacity: OnceLock::new(),
        }
    }

    pub fn project_dirs(&self) -> Result<&app_strategy::Xdg> {
        let project_dirs = self.project_dirs.get_or_try_init(ProjectDirs::new)?;

        Ok(project_dirs)
    }

    fn neobrew_config(&self) -> Result<&NeobrewConfig> {
        self.neobrew_config.get_or_try_init(NeobrewConfig::load)
    }

    pub fn homebrew_config(&self) -> Result<&HomebrewConfig> {
        self.homebrew_config.get_or_try_init(HomebrewConfig::load)
    }

    pub fn client(&self) -> &reqwest::Client {
        self.client.get_or_init(reqwest::Client::new)
    }

    pub fn concurrency_limit(&self) -> usize {
        let concurrency_limit = self.concurrency_limit.get_or_init(|| {
            thread::available_parallelism()
                .unwrap_or(NonZeroUsize::MIN)
                .get()
                .min(Self::MAX_CONCURRENCY)
        });

        *concurrency_limit
    }

    pub fn channel_capacity(&self) -> usize {
        let channel_capacity = self
            .channel_capacity
            .get_or_init(|| self.concurrency_limit() * Self::BUFFER_MULTIPLIER);

        *channel_capacity
    }
}
