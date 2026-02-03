use std::path::PathBuf;

use anyhow::Result;
use figment::{
    Figment,
    providers::{Env, Serialized},
};
use serde::{Deserialize, Serialize};

pub struct Config {
    homebrew: HomebrewConfig,
    neobrew: NeobrewConfig,
}

impl Config {
    pub fn parse() -> Result<Self> {
        let homebrew = Figment::new()
            .merge(Serialized::defaults(HomebrewConfig::default()))
            .merge(Env::prefixed("HOMEBREW_"))
            .extract()?;
        let neobrew = Figment::new()
            .merge(Serialized::defaults(NeobrewConfig::default()))
            .merge(Env::prefixed("NEOBREW_"))
            .extract()?;

        Ok(Self { homebrew, neobrew })
    }
}

#[derive(Serialize, Deserialize)]
struct HomebrewConfig {
    prefix: PathBuf,
}

impl Default for HomebrewConfig {
    fn default() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let prefix = PathBuf::from("/usr/local");

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let prefix = PathBuf::from("/opt/homebrew");

        #[cfg(target_os = "linux")]
        let prefix = PathBuf::from("/home/linuxbrew/.linuxbrew");

        Self { prefix }
    }
}

#[derive(Serialize, Deserialize)]
struct NeobrewConfig {
    prefix: PathBuf,
}

impl Default for NeobrewConfig {
    fn default() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let prefix = PathBuf::from("/usr/local");

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let prefix = PathBuf::from("/opt/homebrew");

        #[cfg(target_os = "linux")]
        let prefix = PathBuf::from("/home/linuxbrew/.linuxbrew");

        Self { prefix }
    }
}
