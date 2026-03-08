use std::{num::NonZeroUsize, sync::LazyLock, thread};

use anyhow::Result;
use clap::ArgMatches;

use self::{
    configs::Config,
    project_dirs::{ChosenAppStrategy, ProjectDirs},
};

mod configs;
mod project_dirs;

static CONCURRENCY_LIMIT: LazyLock<usize> = LazyLock::new(|| {
    thread::available_parallelism()
        .unwrap_or(NonZeroUsize::MIN)
        .get()
        .min(Context::MAX_CONCURRENCY)
});

#[derive(Debug)]
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
    pub fn new(matches: &ArgMatches) -> Result<Self> {
        let this = Self {
            proj_dirs: ProjectDirs::new()?.strategy(),

            config: Config::load(matches)?,

            client: LazyLock::new(reqwest::Client::new),

            concurrency_limit: LazyLock::new(|| *CONCURRENCY_LIMIT),
            channel_capacity: LazyLock::new(|| *CONCURRENCY_LIMIT * Self::BUFFER_MULTIPLIER),
        };

        Ok(this)
    }
}
