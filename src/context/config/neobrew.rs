use std::path::PathBuf;

use anyhow::Result;
use clap::{crate_authors, crate_name};
use etcetera::{AppStrategy, AppStrategyArgs, app_strategy};
use serde::{Deserialize, Serialize};

use super::Config;

#[derive(Serialize, Deserialize)]
pub struct NeobrewConfig {
    home_dir: PathBuf,

    config_dir: PathBuf,
    data_dir: PathBuf,
    cache_dir: PathBuf,
    state_dir: Option<PathBuf>,
    runtime_dir: Option<PathBuf>,
}

impl NeobrewConfig {
    const TOP_LEVEL_DOMAIN: &str = "sh";
    const AUTHOR: &str = "shian15810";
}

impl Config for NeobrewConfig {
    const ENV_PREFIX: &str = "NEOBREW_";

    fn default() -> Result<Self> {
        let author = crate_authors!()
            .split_once(':')
            .and_then(|(head, _)| head.split_once('('))
            .and_then(|(_, tail)| tail.split_once(')'))
            .map_or(Self::AUTHOR, |(head, _)| head);

        let strategy = app_strategy::choose_app_strategy(AppStrategyArgs {
            top_level_domain: Self::TOP_LEVEL_DOMAIN.to_owned(),
            author: author.to_owned(),
            app_name: crate_name!().to_owned(),
        })?;

        let this = Self {
            home_dir: strategy.home_dir().to_owned(),

            config_dir: strategy.config_dir(),
            data_dir: strategy.data_dir(),
            cache_dir: strategy.cache_dir(),
            state_dir: strategy.state_dir(),
            runtime_dir: strategy.runtime_dir(),
        };

        Ok(this)
    }
}
