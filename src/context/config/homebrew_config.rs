use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::context::config::{Config, DEFAULT_PREFIX};

#[derive(Serialize, Deserialize)]
pub struct HomebrewConfig {
    prefix: PathBuf,
}

impl Default for HomebrewConfig {
    fn default() -> Self {
        Self {
            prefix: PathBuf::from(DEFAULT_PREFIX),
        }
    }
}

impl Config for HomebrewConfig {
    const ENV_PREFIX: &'static str = "HOMEBREW_";
}
