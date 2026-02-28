use anyhow::Result;
use clap::{ArgMatches, ColorChoice};
use clap_verbosity_flag::{Verbosity, VerbosityFilter};
use figment::{
    Figment,
    Metadata,
    Profile,
    Provider,
    providers::Serialized,
    value::{Dict, Map},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_with::{DisplayFromStr, serde_as};

use self::{
    cli::CliConfig,
    global_env::GlobalEnvConfig,
    homebrew_env::HomebrewEnvConfig,
    neobrew_env::NeobrewEnvConfig,
};

mod cli;
mod global_env;
mod homebrew_env;
mod neobrew_env;

struct FigmentProvider<ProviderConf>(ProviderConf);

impl<ProviderConf: ProviderConfig> Provider for FigmentProvider<ProviderConf> {
    fn metadata(&self) -> Metadata {
        self.0.metadata()
    }

    fn data(&self) -> figment::Result<Map<Profile, Dict>> {
        self.0.data()
    }
}

trait ProviderConfig: Sized {
    const METADATA_NAME: &str;

    fn metadata(&self) -> Metadata {
        Metadata::named(Self::METADATA_NAME)
    }

    fn data(&self) -> figment::Result<Map<Profile, Dict>>;

    fn into_provider(self) -> FigmentProvider<Self> {
        FigmentProvider(self)
    }
}

trait EnvConfig: DeserializeOwned {
    const ENV_PREFIX: &str;

    fn from_env() -> Result<Self> {
        let this = Self::default_from_env()?;

        Ok(this)
    }

    fn default_from_env() -> Result<Self> {
        let this: Self = envy::prefixed(Self::ENV_PREFIX).from_env()?;

        Ok(this)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub verbosity_filter: VerbosityFilter,

    #[serde_as(as = "DisplayFromStr")]
    pub color_choice: ColorChoice,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            verbosity_filter: <Verbosity>::default().filter(),

            color_choice: ColorChoice::default(),
        }
    }
}

impl Config {
    pub fn load(matches: &ArgMatches) -> Result<Self> {
        let this: Self = Self::figment(matches)?.extract()?;

        Ok(this)
    }

    fn figment(matches: &ArgMatches) -> Result<Figment> {
        let figment = Figment::new()
            .merge(Serialized::defaults(Self::default()))
            .merge(GlobalEnvConfig::from_env()?.into_provider())
            .merge(HomebrewEnvConfig::from_env()?.into_provider())
            .merge(NeobrewEnvConfig::from_env()?.into_provider())
            .merge(CliConfig::from_arg_matches(matches).into_provider());

        Ok(figment)
    }
}
