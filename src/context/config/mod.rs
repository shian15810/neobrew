use color_eyre::eyre::Result;
use figment::{
    Figment,
    providers::{Env, Serialized},
};
use serde::{Serialize, de::DeserializeOwned};

pub mod homebrew_config;
pub mod neobrew_config;

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const DEFAULT_PREFIX: &str = "/usr/local";

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const DEFAULT_PREFIX: &str = "/opt/homebrew";

#[cfg(target_os = "linux")]
const DEFAULT_PREFIX: &str = "/home/linuxbrew/.linuxbrew";

pub trait Config: Default + Serialize + DeserializeOwned {
    const ENV_PREFIX: &'static str;

    fn load() -> Result<Self> {
        let config = Figment::new()
            .merge(Serialized::defaults(Self::default()))
            .merge(Env::prefixed(Self::ENV_PREFIX))
            .extract()?;

        Ok(config)
    }
}
