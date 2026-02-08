use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Config, DEFAULT_PREFIX};

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
    const ENV_PREFIX: &str = "HOMEBREW_";
}
