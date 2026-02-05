use std::sync::Arc;

use color_eyre::eyre::Result;
use moka::future::Cache;
use once_cell::sync::OnceCell as OnceLock;

use self::config::{Config, HomebrewConfig, NeobrewConfig};
use crate::package::{Cask, Formula};

mod config;

type FormulaRegistry = Cache<String, Arc<Formula>>;
type CaskRegistry = Cache<String, Arc<Cask>>;

pub struct Context {
    homebrew_config: OnceLock<HomebrewConfig>,
    neobrew_config: OnceLock<NeobrewConfig>,

    http_client: OnceLock<reqwest::Client>,

    formula_registry: OnceLock<FormulaRegistry>,
    cask_registry: OnceLock<CaskRegistry>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            homebrew_config: OnceLock::new(),
            neobrew_config: OnceLock::new(),

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

    pub fn http_client(&self) -> &reqwest::Client {
        self.http_client.get_or_init(reqwest::Client::new)
    }

    pub fn formula_registry(&self) -> &FormulaRegistry {
        self.formula_registry
            .get_or_init(|| Cache::builder().build())
    }

    pub fn cask_registry(&self) -> &CaskRegistry {
        self.cask_registry.get_or_init(|| Cache::builder().build())
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
