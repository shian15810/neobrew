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
    homebrew_config: OnceLock<HomebrewConfig>,
    neobrew_config: OnceLock<NeobrewConfig>,

    project_dirs: OnceLock<ProjectDirs>,

    max_concurrency: OnceLock<usize>,
    http_client: OnceLock<reqwest::Client>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            homebrew_config: OnceLock::new(),
            neobrew_config: OnceLock::new(),

            project_dirs: OnceLock::new(),

            max_concurrency: OnceLock::new(),
            http_client: OnceLock::new(),
        }
    }

    pub fn homebrew_config(&self) -> Result<&HomebrewConfig> {
        self.homebrew_config.get_or_try_init(HomebrewConfig::load)
    }

    fn neobrew_config(&self) -> Result<&NeobrewConfig> {
        self.neobrew_config.get_or_try_init(NeobrewConfig::load)
    }

    fn project_dirs(&self) -> Result<&app_strategy::Xdg> {
        let project_dirs = self.project_dirs.get_or_try_init(ProjectDirs::new)?;

        Ok(project_dirs)
    }

    pub fn max_concurrency(&self) -> &usize {
        self.max_concurrency.get_or_init(|| 16)
    }

    pub fn http_client(&self) -> &reqwest::Client {
        self.http_client.get_or_init(reqwest::Client::new)
    }
}
