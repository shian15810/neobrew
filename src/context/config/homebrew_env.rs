use anyhow::anyhow;
use clap::ColorChoice;
use clap_verbosity_flag::VerbosityFilter;
use figment::{
    Profile,
    value::{Dict, Map, Value},
};
use indoc::formatdoc;
use serde::{Deserialize, Deserializer};
use serde_with::{DeserializeAs, serde_as};

use super::{EnvConfig, ProviderConfig};

#[cfg_attr(not(debug_assertions), visibility::make(pub(in super::super)))]
#[serde_as]
#[derive(Deserialize)]
pub(super) struct HomebrewEnvConfig {
    prefix: String,

    #[serde_as(as = "Option<HomebrewBoolFromStr>")]
    debug: Option<bool>,

    #[serde_as(as = "Option<HomebrewBoolFromStr>")]
    verbose: Option<bool>,

    #[serde_as(as = "Option<HomebrewBoolFromStr>")]
    no_color: Option<bool>,

    #[serde_as(as = "Option<HomebrewBoolFromStr>")]
    color: Option<bool>,
}

impl EnvConfig for HomebrewEnvConfig {
    const ENV_PREFIX: &str = "HOMEBREW_";

    fn from_env() -> anyhow::Result<Self> {
        let this = Self::default_from_env()?;

        Self::ensure_default_prefix(&this)?;

        Ok(this)
    }
}

impl HomebrewEnvConfig {
    #[cfg_attr(not(debug_assertions), visibility::make(pub(in super::super)))]
    const DEFAULT_PREFIX: &str = cfg_select! {
        target_os = "macos" => cfg_select! {
            target_arch = "aarch64" => "/opt/homebrew",
            target_arch = "x86_64" => "/usr/local",
        },
        target_os = "linux" => "/home/linuxbrew/.linuxbrew",
    };

    fn ensure_default_prefix(&self) -> anyhow::Result<()> {
        let prefix = &self.prefix;

        if prefix == Self::DEFAULT_PREFIX {
            return Ok(());
        }

        let err = anyhow!(formatdoc! {r#"
            Unsupported `HOMEBREW_PREFIX` detected: "{prefix}"

            Neobrew requires the default prefix to use pre-compiled bottles and casks:

                - macOS (Apple Silicon)     -->     "/opt/homebrew"
                - macOS (Intel x86_64)      -->     "/usr/local"
                - Linux                     -->     "/home/linuxbrew/.linuxbrew"

            The default prefix is essential for Neobrew's high-performance guarantees,
            seamless developer experience, and smooth interoperability with your local
            Homebrew installation - so you can use `nbrew` and `brew` interchangeably.

            See https://docs.brew.sh/Installation"#,
        });

        Err(err)
    }
}

impl ProviderConfig for HomebrewEnvConfig {
    const METADATA_NAME: &str = "Homebrew environment variable(s)";

    fn data(&self) -> figment::Result<Map<Profile, Dict>> {
        let verbosity_filter = match (self.debug, self.verbose) {
            (Some(true), _) => Some(VerbosityFilter::Debug),
            (_, Some(true)) => Some(VerbosityFilter::Info),
            _ => None,
        };
        let verbosity_filter = verbosity_filter.map(Value::serialize).transpose()?;

        let color_choice = match (self.no_color, self.color) {
            (Some(true), _) => Some(ColorChoice::Never),
            (_, Some(true)) => Some(ColorChoice::Always),
            _ => None,
        };
        let color_choice = color_choice.map(|val| val.to_string()).map(Value::from);

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

struct HomebrewBoolFromStr;

impl<'de> DeserializeAs<'de, bool> for HomebrewBoolFromStr {
    fn deserialize_as<D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
        let value = homebrew_bool_from_str(deserializer)?;

        Ok(value)
    }
}

fn homebrew_bool_from_str<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    const FALSY_VALUES: &[&str] = &["false", "no", "off", "nil", "0"];

    let value = String::deserialize(deserializer)?;

    if value.is_empty() {
        return Ok(false);
    }

    let is_falsy_value = FALSY_VALUES
        .iter()
        .any(|&falsy_value| falsy_value.eq_ignore_ascii_case(&value));

    let is_truthy_value = !is_falsy_value;

    Ok(is_truthy_value)
}
