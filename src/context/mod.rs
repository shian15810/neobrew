use color_eyre::eyre::Result;
use once_cell::sync::OnceCell as OnceLock;
use reqwest::Client;

use crate::context::config::Config;

mod config;

pub struct Context {
    client: OnceLock<Client>,
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
            client: OnceLock::new(),
            config: OnceLock::new(),
        }
    }

    pub fn client(&self) -> &Client {
        self.client.get_or_init(Client::new)
    }

    fn config(&self) -> Result<&Config> {
        self.config.get_or_try_init(Config::parse)
    }
}
