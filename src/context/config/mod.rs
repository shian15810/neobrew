use anyhow::Result;
use figment::{
    Figment,
    providers::{Env, Serialized},
};
use serde::{Serialize, de::DeserializeOwned};

pub use self::{homebrew::HomebrewConfig, neobrew::NeobrewConfig};

mod homebrew;
mod neobrew;

pub trait Config: Default + Serialize + DeserializeOwned {
    const ENV_PREFIX: &str;

    cfg_if::cfg_if! {
        if #[cfg(all(target_os = "macos", target_arch = "x86_64"))] {
            const DEFAULT_PREFIX: &str = "/usr/local";
        } else if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
            const DEFAULT_PREFIX: &str = "/opt/homebrew";
        } else if #[cfg(target_os = "linux")] {
            const DEFAULT_PREFIX: &str = "/home/linuxbrew/.linuxbrew";
        } else {
            compile_error!("This crate only supports macOS and Linux.");
        }
    }

    fn load() -> Result<Self> {
        let config = Figment::new()
            .merge(Serialized::defaults(Self::default()))
            .merge(Env::prefixed(Self::ENV_PREFIX))
            .extract()?;

        Ok(config)
    }
}
