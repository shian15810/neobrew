use std::{num::NonZeroUsize, sync::LazyLock, thread};

use anyhow::Result;
use clap::ArgMatches;
use proc_exit::prelude::*;

use self::{
    config::Config,
    project_dirs::{ChosenAppStrategy, ProjectDirs},
};

mod config;
mod project_dirs;

static CONCURRENCY_LIMIT: LazyLock<usize> = LazyLock::new(|| {
    thread::available_parallelism()
        .unwrap_or(NonZeroUsize::MIN)
        .get()
        .min(Context::MAX_CONCURRENCY)
});

pub struct Context {
    pub(super) proj_dirs: ChosenAppStrategy,

    pub(super) config: Config,

    pub(super) client: LazyLock<reqwest::Client>,

    pub(super) concurrency_limit: LazyLock<usize>,
    pub(super) channel_capacity: LazyLock<usize>,
}

impl Context {
    const MAX_CONCURRENCY: usize = 1 << 4;
    const BUFFER_MULTIPLIER: usize = 1 << 4;

    #[allow(clippy::missing_errors_doc)]
    pub fn new(matches: &ArgMatches) -> Result<Self, proc_exit::Exit> {
        let proj_dirs = ProjectDirs::new()
            .with_code(proc_exit::sysexits::OS_ERR)?
            .strategy();

        let config = Config::load(matches).with_code(proc_exit::sysexits::CONFIG_ERR)?;

        let this = Self {
            proj_dirs,

            config,

            client: LazyLock::new(reqwest::Client::new),

            concurrency_limit: LazyLock::new(|| *CONCURRENCY_LIMIT),
            channel_capacity: LazyLock::new(|| *CONCURRENCY_LIMIT * Self::BUFFER_MULTIPLIER),
        };

        Ok(this)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
