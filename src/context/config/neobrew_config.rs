use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::context::config::{Config, DEFAULT_PREFIX};

#[derive(Serialize, Deserialize)]
pub struct NeobrewConfig {
    prefix: PathBuf,
}

impl Default for NeobrewConfig {
    fn default() -> Self {
        Self {
            prefix: PathBuf::from(DEFAULT_PREFIX),
        }
    }
}

impl Config for NeobrewConfig {
    const ENV_PREFIX: &'static str = "NEOBREW_";
}
