use serde::{Deserialize, Serialize};

use super::Config;

#[derive(Serialize, Deserialize)]
pub struct NeobrewConfig {}

impl Default for NeobrewConfig {
    fn default() -> Self {
        Self {}
    }
}

impl Config for NeobrewConfig {
    const ENV_PREFIX: &str = "NEOBREW_";
}
