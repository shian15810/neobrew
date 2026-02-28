use clap::ColorChoice;
use figment::{
    Profile,
    value::{Dict, Map, Value},
};
use serde::Deserialize;
use serde_with::{NoneAsEmptyString, serde_as};

use super::{EnvConfig, ProviderConfig};

#[serde_as]
#[derive(Deserialize)]
pub struct GlobalEnvConfig {
    #[serde(default)]
    #[serde_as(as = "NoneAsEmptyString")]
    no_color: Option<String>,

    #[serde(default)]
    #[serde_as(as = "NoneAsEmptyString")]
    force_color: Option<String>,

    #[serde(default)]
    #[serde_as(as = "NoneAsEmptyString")]
    clicolor_force: Option<String>,

    #[serde(default)]
    #[serde_as(as = "NoneAsEmptyString")]
    clicolor: Option<String>,
}

impl EnvConfig for GlobalEnvConfig {
    const ENV_PREFIX: &str = "";
}

impl ProviderConfig for GlobalEnvConfig {
    const METADATA_NAME: &str = "Global environment variable(s)";

    fn data(&self) -> figment::Result<Map<Profile, Dict>> {
        let color_choice = match (
            self.no_color.as_deref(),
            self.force_color.as_deref().or(self.clicolor_force.as_deref()),
            self.clicolor.as_deref(),
        ) {
            (Some(_), _, _) => Some(ColorChoice::Never),
            (_, Some(_), _) => Some(ColorChoice::Always),
            (_, _, Some(_)) => Some(ColorChoice::Auto),
            _ => None,
        };
        let color_choice = color_choice.map(|val| val.to_string()).map(Value::from);

        let dict = [color_choice.map(|val| ("color_choice", val))];
        let dict = dict
            .into_iter()
            .flatten()
            .map(|(key, val)| (key.to_owned(), val))
            .collect::<Dict>();

        let map = Map::from([(Profile::Default, dict)]);

        Ok(map)
    }
}
