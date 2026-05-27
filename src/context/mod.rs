mod config;
pub(crate) mod dirs;

use std::{num::NonZeroUsize, sync::LazyLock, thread};

use clap::ArgMatches;
use oci_client::{Client, client::ClientConfig};
use proc_exit::prelude::*;
use tokio::sync::Semaphore;

use self::{
    config::Config,
    dirs::{HomebrewDirs, NeobrewDirs},
};

const MAX_CONCURRENCY: usize = 1 << 4;

static CONCURRENCY_LIMIT: LazyLock<usize> = LazyLock::new(|| {
    thread::available_parallelism()
        .unwrap_or(NonZeroUsize::MIN)
        .get()
        .min(MAX_CONCURRENCY)
});

const BUFFER_MULTIPLIER: usize = 1 << 4;

#[expect(clippy::module_name_repetitions)]
pub struct Context {
    pub(crate) config: Config,

    pub(crate) homebrew_dirs: HomebrewDirs,
    pub(crate) neobrew_dirs: NeobrewDirs,

    pub(crate) client: reqwest::Client,
    pub(crate) oci_client: Client,

    pub(crate) semaphore: Semaphore,

    pub(crate) concurrency_limit: usize,
    pub(crate) channel_capacity: usize,
}

impl Context {
    #[expect(clippy::missing_errors_doc)]
    pub fn new(matches: &ArgMatches) -> Result<Self, proc_exit::Exit> {
        let config = Config::load(matches);
        let config = config.with_code(proc_exit::sysexits::CONFIG_ERR)?;

        let homebrew_dirs = HomebrewDirs::new();
        let homebrew_dirs = homebrew_dirs.with_code(proc_exit::sysexits::OS_ERR)?;

        let neobrew_dirs = NeobrewDirs::new();
        let neobrew_dirs = neobrew_dirs.with_code(proc_exit::sysexits::OS_ERR)?;

        let this = Self {
            config,

            homebrew_dirs,
            neobrew_dirs,

            client: reqwest::Client::new(),
            oci_client: Client::new(ClientConfig::default()),

            semaphore: Semaphore::new(*CONCURRENCY_LIMIT),

            concurrency_limit: *CONCURRENCY_LIMIT,
            channel_capacity: CONCURRENCY_LIMIT.saturating_mul(BUFFER_MULTIPLIER),
        };

        Ok(this)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
