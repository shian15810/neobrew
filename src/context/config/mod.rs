use anyhow::Result;
use figment::{
    Figment,
    providers::{Env, Serialized},
};
use serde::{Serialize, de::DeserializeOwned};

pub use self::{homebrew::HomebrewConfig, neobrew::NeobrewConfig};

mod homebrew;
mod neobrew;

pub trait Config: Serialize + DeserializeOwned {
    const ENV_PREFIX: &str;

    fn default() -> Result<Self>;

    fn load() -> Result<Self> {
        let config = Figment::new()
            .merge(Serialized::defaults(Self::default()?))
            .merge(Env::prefixed(Self::ENV_PREFIX))
            .extract()?;

        Ok(config)
    }
}
