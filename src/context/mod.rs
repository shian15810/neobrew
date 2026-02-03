use anyhow::Result;
use once_cell::sync::OnceCell as OnceLock;

use crate::context::config::Config;

mod config;

pub struct Context {
    config: OnceLock<Config>,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            config: OnceLock::new(),
        }
    }

    fn config(&self) -> Result<&Config> {
        self.config.get_or_try_init(Config::parse)
    }
}
