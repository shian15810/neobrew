use std::sync::Arc;

use color_eyre::eyre::Result;
use moka::future::Cache;
use once_cell::sync::OnceCell as OnceLock;
use reqwest::Client;

use self::config::{Config, homebrew_config::HomebrewConfig, neobrew_config::NeobrewConfig};
use crate::package::{cask::Cask, formula::Formula};

mod config;

type FormulaRegistry = Cache<String, Arc<Formula>>;
type CaskRegistry = Cache<String, Arc<Cask>>;

pub struct Context {
    client: OnceLock<Client>,

    homebrew_config: OnceLock<HomebrewConfig>,
    neobrew_config: OnceLock<NeobrewConfig>,

    formula_registry: OnceLock<FormulaRegistry>,
    cask_registry: OnceLock<CaskRegistry>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            client: OnceLock::new(),

            homebrew_config: OnceLock::new(),
            neobrew_config: OnceLock::new(),

            formula_registry: OnceLock::new(),
            cask_registry: OnceLock::new(),
        }
    }

    pub fn client(&self) -> &Client {
        self.client.get_or_init(Client::new)
    }

    fn homebrew_config(&self) -> Result<&HomebrewConfig> {
        self.homebrew_config.get_or_try_init(HomebrewConfig::load)
    }

    fn neobrew_config(&self) -> Result<&NeobrewConfig> {
        self.neobrew_config.get_or_try_init(NeobrewConfig::load)
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
