use std::sync::Arc;

use anyhow::Result;
use foyer::{HybridCache, HybridCacheBuilder};
use once_cell::sync::OnceCell as OnceLock;
use tokio::sync::OnceCell;

use self::config::{Config, HomebrewConfig, NeobrewConfig};
use crate::package::{Cask, Formula};

mod config;

type FormulaRegistry = HybridCache<String, Arc<Formula>>;
type CaskRegistry = HybridCache<String, Arc<Cask>>;

pub struct Context {
    homebrew_config: OnceLock<HomebrewConfig>,
    neobrew_config: OnceLock<NeobrewConfig>,

    http_client: OnceLock<reqwest::Client>,

    formula_registry: OnceCell<FormulaRegistry>,
    cask_registry: OnceCell<CaskRegistry>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            homebrew_config: OnceLock::new(),
            neobrew_config: OnceLock::new(),

            http_client: OnceLock::new(),

            formula_registry: OnceCell::new(),
            cask_registry: OnceCell::new(),
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

    pub async fn formula_registry(&self) -> Result<&FormulaRegistry> {
        let hybrid = self
            .formula_registry
            .get_or_try_init(|| {
                HybridCacheBuilder::new()
                    .memory(usize::MAX)
                    .storage()
                    .build()
            })
            .await?;

        Ok(hybrid)
    }

    pub async fn cask_registry(&self) -> Result<&CaskRegistry> {
        let hybrid = self
            .cask_registry
            .get_or_try_init(|| {
                HybridCacheBuilder::new()
                    .memory(usize::MAX)
                    .storage()
                    .build()
            })
            .await?;

        Ok(hybrid)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
