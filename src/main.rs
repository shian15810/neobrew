use std::{collections::HashMap, ffi::OsString, path::PathBuf, process::Command};

use clap::{Args, Parser, Subcommand};
use enum_dispatch::enum_dispatch;
use figment::{
    Figment,
    providers::{Env, Serialized},
};
use serde::{Deserialize, Serialize};

#[enum_dispatch(Internal)]
trait Runner {
    fn run(&self, config: &Config);
}

#[derive(Args)]
struct Install {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,
}

impl Runner for Install {
    fn run(&self, config: &Config) {
        println!("Install packages: {:?}", self.packages);
    }
}

#[derive(Args)]
struct Uninstall {
    #[arg(value_name = "FORMULA|CASK")]
    packages: Vec<String>,
}

impl Runner for Uninstall {
    fn run(&self, config: &Config) {
        println!("Uninstall packages: {:?}", self.packages);
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
    fn parse() -> Self {
        Self {
            homebrew: Figment::from(Serialized::defaults(HomebrewConfig::default()))
                .merge(Env::prefixed("HOMEBREW_"))
                .extract()
                .unwrap(),
            neobrew: Figment::from(Serialized::defaults(NeobrewConfig::default()))
                .merge(Env::prefixed("NEOBREW_"))
                .extract()
                .unwrap(),
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let config = Config::parse();

    match &cli.command {
        Commands::Internal(cmd) => cmd.run(&config),

        Commands::External(args) => match Command::new("brew")
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
        {
            Ok(status) => std::process::exit(status.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("Failed to execute brew: {e}");
                std::process::exit(1);
            },
        },
    }
}
