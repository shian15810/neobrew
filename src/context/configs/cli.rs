use clap::{ArgMatches, ColorChoice, parser::ValueSource};
use clap_verbosity_flag::Verbosity;
use figment::{
    Profile,
    value::{Dict, Map, Value},
};

use super::ProviderConfig;

pub struct CliConfig {
    verbosity: Option<Verbosity>,

    color: Option<ColorChoice>,
}

impl CliConfig {
    pub fn from_arg_matches(matches: &ArgMatches) -> Self {
        let is_from_cli = |id| matches.value_source(id) == Some(ValueSource::CommandLine);

        let verbosity = (is_from_cli("verbose") || is_from_cli("quiet")).then(|| {
            let verbose = matches.get_count("verbose");

            let quiet = matches.get_count("quiet");

            Verbosity::new(verbose, quiet)
        });

        let color = is_from_cli("color")
            .then(|| matches.get_one::<ColorChoice>("color"))
            .flatten()
            .copied();

        Self {
            verbosity,
            color,
        }
    }
}

impl ProviderConfig for CliConfig {
    const METADATA_NAME: &str = "Command-line argument(s)";

    fn data(&self) -> figment::Result<Map<Profile, Dict>> {
        let verbosity_filter = self
            .verbosity
            .map(|val| val.filter())
            .map(Value::serialize)
            .transpose()?;

        let color_choice = self.color.map(|val| val.to_string()).map(Value::from);

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
