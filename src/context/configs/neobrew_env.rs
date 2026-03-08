use clap::ColorChoice;
use clap_verbosity_flag::VerbosityFilter;
use figment::{
    Profile,
    value::{Dict, Map, Value},
};
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};

use super::{EnvConfig, ProviderConfig};

#[serde_as]
#[derive(Deserialize)]
pub(super) struct NeobrewEnvConfig {
    verbosity_filter: Option<VerbosityFilter>,

    #[serde_as(as = "Option<DisplayFromStr>")]
    color_choice: Option<ColorChoice>,
}

impl EnvConfig for NeobrewEnvConfig {
    const ENV_PREFIX: &str = "NEOBREW_";
}

impl ProviderConfig for NeobrewEnvConfig {
    const METADATA_NAME: &str = "Neobrew environment variable(s)";

    fn data(&self) -> figment::Result<Map<Profile, Dict>> {
        let verbosity_filter = self.verbosity_filter.map(Value::serialize).transpose()?;

        let color_choice = self
            .color_choice
            .map(|val| val.to_string())
            .map(Value::from);

        let dict = [
            verbosity_filter.map(|val| ("verbosity_filter", val)),
            color_choice.map(|val| ("color_choice", val)),
        ];
        let dict = dict
            .into_iter()
            .flatten()
            .map(|(key, val)| (key.to_owned(), val))
            .collect::<Dict>();

        let map = Map::from([(Profile::Default, dict)]);

        Ok(map)
    }
}
