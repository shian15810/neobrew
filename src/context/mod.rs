use std::sync::Arc;

use anyhow::Result;
use foyer::{Cache, CacheBuilder};
use once_cell::sync::OnceCell as OnceLock;

use self::config::{Config, HomebrewConfig, NeobrewConfig};
use crate::package::{Cask, Formula};

mod config;

pub type FormulaRegistry = Cache<String, Arc<Formula>>;
pub type CaskRegistry = Cache<String, Arc<Cask>>;

pub struct Context {
    homebrew_config: OnceLock<HomebrewConfig>,
    neobrew_config: OnceLock<NeobrewConfig>,

    max_concurrency: OnceLock<usize>,
    http_client: OnceLock<reqwest::Client>,

    formula_registry: OnceLock<FormulaRegistry>,
    cask_registry: OnceLock<CaskRegistry>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            homebrew_config: OnceLock::new(),
            neobrew_config: OnceLock::new(),

            max_concurrency: OnceLock::new(),
            http_client: OnceLock::new(),

            formula_registry: OnceLock::new(),
            cask_registry: OnceLock::new(),
        }
    }

    fn homebrew_config(&self) -> Result<&HomebrewConfig> {
        self.homebrew_config.get_or_try_init(HomebrewConfig::load)
    }

    fn neobrew_config(&self) -> Result<&NeobrewConfig> {
        self.neobrew_config.get_or_try_init(NeobrewConfig::load)
    }

    pub fn max_concurrency(&self) -> &usize {
        self.max_concurrency.get_or_init(|| 16)
    }

    pub fn http_client(&self) -> &reqwest::Client {
        self.http_client.get_or_init(reqwest::Client::new)
    }

    pub fn formula_registry(&self) -> &FormulaRegistry {
        self.formula_registry
            .get_or_init(|| CacheBuilder::new(usize::MAX).build())
    }

    pub fn cask_registry(&self) -> &CaskRegistry {
        self.cask_registry
            .get_or_init(|| CacheBuilder::new(usize::MAX).build())
    }
}
