use anyhow::Result;
use cfg_if::cfg_if;
use clap::{crate_authors, crate_name};
use etcetera::app_strategy::{self, AppStrategyArgs};

pub(super) struct ProjectDirs {
    strategy: ChosenAppStrategy,
}

impl ProjectDirs {
    const TOP_LEVEL_DOMAIN: &str = "sh";
    const AUTHOR: &str = "shian15810";

    pub(super) fn new() -> Result<Self> {
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
            strategy,
        };

        Ok(this)
    }

    pub(super) fn strategy(self) -> ChosenAppStrategy {
        self.strategy
    }
}

cfg_if! {
    if #[cfg(target_os = "windows")] {
        pub(super) type ChosenAppStrategy = app_strategy::Windows;
    } else if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        pub(super) type ChosenAppStrategy = app_strategy::Xdg;
    } else {
        pub(super) type ChosenAppStrategy = app_strategy::Xdg;
    }
}
