use std::{num::NonZeroUsize, sync::LazyLock, thread};

use clap::ArgMatches;
use oci_client::{Client, client::ClientConfig};
use proc_exit::prelude::*;
use tokio::sync::Semaphore;

pub(crate) use self::dirs::ProjectDirs;
use self::{
    config::Config,
    dirs::{HomebrewDirs, NeobrewDirs},
};

mod config;
mod dirs;

#[expect(clippy::module_name_repetitions)]
pub struct Context {
    pub(crate) config: Config,

    pub(crate) homebrew_dirs: HomebrewDirs,
    pub(crate) neobrew_dirs: NeobrewDirs,

    pub(crate) client: LazyLock<reqwest::Client>,
    pub(crate) oci_client: LazyLock<Client>,

    pub(crate) semaphore: LazyLock<Semaphore>,

    pub(crate) concurrency_limit: LazyLock<usize>,
    pub(crate) channel_capacity: LazyLock<usize>,
}

impl Context {
    const MAX_CONCURRENCY: usize = 1 << 4;
    const BUFFER_MULTIPLIER: usize = 1 << 4;

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

            client: LazyLock::new(reqwest::Client::new),
            oci_client: LazyLock::new(|| Client::new(ClientConfig::default())),

            semaphore: LazyLock::new(|| Semaphore::new(*CONCURRENCY_LIMIT)),

            concurrency_limit: LazyLock::new(|| *CONCURRENCY_LIMIT),
            channel_capacity: LazyLock::new(|| {
                CONCURRENCY_LIMIT.saturating_mul(Self::BUFFER_MULTIPLIER)
            }),
        };

        Ok(this)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}

static CONCURRENCY_LIMIT: LazyLock<usize> = LazyLock::new(|| {
    thread::available_parallelism()
        .unwrap_or(NonZeroUsize::MIN)
        .get()
        .min(Context::MAX_CONCURRENCY)
});
