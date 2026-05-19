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

#[cfg_attr(not(debug_assertions), visibility::make(pub(super)))]
use self::homebrew_env::HomebrewEnvConfig;
use self::{cli::CliConfig, global_env::GlobalEnvConfig, neobrew_env::NeobrewEnvConfig};

mod cli;
mod global_env;
mod homebrew_env;
mod neobrew_env;

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub(crate) verbosity_filter: VerbosityFilter,

    #[serde_as(as = "DisplayFromStr")]
    pub(crate) color_choice: ColorChoice,
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
    pub(super) fn load(matches: &ArgMatches) -> Result<Self> {
        let this = Self::figment(matches)?;
        let this: Self = this.extract()?;

        Ok(this)
    }

    fn figment(matches: &ArgMatches) -> Result<Figment> {
        let default_config_provider = Self::default();
        let default_config_provider = default_config_provider.into_provider();

        let global_env_config_provider = GlobalEnvConfig::from_env()?;
        let global_env_config_provider = global_env_config_provider.into_provider();

        let homebrew_env_config_provider = HomebrewEnvConfig::from_env()?;
        let homebrew_env_config_provider = homebrew_env_config_provider.into_provider();

        let neobrew_env_config_provider = NeobrewEnvConfig::from_env()?;
        let neobrew_env_config_provider = neobrew_env_config_provider.into_provider();

        let cli_config_provider = CliConfig::from_arg_matches(matches);
        let cli_config_provider = cli_config_provider.into_provider();

        let figment = Figment::new()
            .merge(default_config_provider)
            .merge(global_env_config_provider)
            .merge(homebrew_env_config_provider)
            .merge(neobrew_env_config_provider)
            .merge(cli_config_provider);

        Ok(figment)
    }

    fn into_provider(self) -> Serialized<Self> {
        Serialized::defaults(self)
    }

    pub fn verbosity_filter(&self) -> &VerbosityFilter {
        &self.verbosity_filter
    }
}

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

    #[expect(clippy::result_large_err)]
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
        let this = envy::prefixed(Self::ENV_PREFIX);
        let this: Self = this.from_env()?;

        Ok(this)
    }
}
