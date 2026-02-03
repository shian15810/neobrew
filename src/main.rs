use std::{collections::HashMap, ffi::OsString, io, path::PathBuf, process::ExitStatus};

use anyhow::Result;
use async_trait::async_trait;
use clap::{Args, Parser, Subcommand};
use enum_dispatch::enum_dispatch;
use figment::{
    Figment,
    providers::{Env, Serialized},
};
use proc_exit::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[async_trait]
#[enum_dispatch(Internal)]
trait Runner {
    async fn run(&self, config: &Config) -> Result<()>;
}

#[derive(Args)]
struct Install {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,
}

#[async_trait]
impl Runner for Install {
    async fn run(&self, config: &Config) -> Result<()> {
        println!("Install packages: {:?}", self.packages);

        Ok(())
    }
}

#[derive(Args)]
struct Uninstall {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,
}

#[async_trait]
impl Runner for Uninstall {
    async fn run(&self, config: &Config) -> Result<()> {
        println!("Uninstall packages: {:?}", self.packages);

        Ok(())
    }
}

#[derive(Parser)]
#[command(bin_name = "nbrew", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[enum_dispatch]
#[derive(Subcommand)]
enum Internal {
    Install(Install),
    Uninstall(Uninstall),
}

#[derive(Subcommand)]
enum Commands {
    #[command(flatten)]
    Internal(Internal),

    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(Serialize, Deserialize)]
struct HomebrewConfig {
    prefix: PathBuf,
}

impl Default for HomebrewConfig {
    fn default() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let prefix = PathBuf::from("/usr/local");

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let prefix = PathBuf::from("/opt/homebrew");

        #[cfg(target_os = "linux")]
        let prefix = PathBuf::from("/home/linuxbrew/.linuxbrew");

        Self { prefix }
    }
}

#[derive(Serialize, Deserialize)]
struct NeobrewConfig {
    prefix: PathBuf,
}

impl Default for NeobrewConfig {
    fn default() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let prefix = PathBuf::from("/usr/local");

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let prefix = PathBuf::from("/opt/homebrew");

        #[cfg(target_os = "linux")]
        let prefix = PathBuf::from("/home/linuxbrew/.linuxbrew");

        Self { prefix }
    }
}

struct Config {
    homebrew: HomebrewConfig,
    neobrew: NeobrewConfig,
}

impl Config {
    fn parse() -> Result<Self> {
        let homebrew = Figment::new()
            .merge(Serialized::defaults(HomebrewConfig::default()))
            .merge(Env::prefixed("HOMEBREW_"))
            .extract()?;
        let neobrew = Figment::new()
            .merge(Serialized::defaults(NeobrewConfig::default()))
            .merge(Env::prefixed("NEOBREW_"))
            .extract()?;

        Ok(Self { homebrew, neobrew })
    }
}

async fn run_brew(args: &Vec<OsString>) -> io::Result<ExitStatus> {
    Command::new("brew")
        .args(args)
        .envs(HashMap::from([
            ("HOMEBREW_NO_ANALYTICS", "1"),
            ("HOMEBREW_NO_AUTOREMOVE", "1"),
            ("HOMEBREW_NO_AUTO_UPDATE", "1"),
            ("HOMEBREW_NO_ENV_HINTS", "1"),
            ("HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK", "1"),
            ("HOMEBREW_NO_INSTALL_CLEANUP", "1"),
            ("HOMEBREW_NO_INSTALL_UPGRADE", "1"),
        ]))
        .status()
        .await
}

async fn run() -> proc_exit::ExitResult {
    let cli = Cli::parse();
    let config = Config::parse().with_code(proc_exit::sysexits::CONFIG_ERR)?;

    match &cli.command {
        Commands::Internal(cmd) => cmd
            .run(&config)
            .await
            .with_code(proc_exit::sysexits::SOFTWARE_ERR),

        Commands::External(args) => {
            let exit_status = run_brew(args).await.to_sysexits()?;

            proc_exit::Code::from_status(exit_status).ok()?;
            proc_exit::Code::SUCCESS.ok()
        },
    }
}

#[tokio::main]
async fn main() {
    let result = run().await;

    proc_exit::exit(result);
}
