use anyhow::Result;
use clap::{crate_authors, crate_name};
use etcetera::{AppStrategyArgs, app_strategy};

pub struct ProjectDirs {
    strategy: app_strategy::Xdg,
}

impl ProjectDirs {
    const TOP_LEVEL_DOMAIN: &str = "sh";
    const AUTHOR: &str = "shian15810";

    pub fn new() -> Result<Self> {
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

        let this = Self { strategy };

        Ok(this)
    }

    pub fn strategy(&self) -> &app_strategy::Xdg {
        &self.strategy
    }
}
