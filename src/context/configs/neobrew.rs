use serde::{Deserialize, Serialize};

use super::Config;

#[derive(Default, Serialize, Deserialize)]
pub struct NeobrewConfig;

impl NeobrewConfig {}

impl Config for NeobrewConfig {
    const ENV_PREFIX: &str = "NEOBREW_";
}
