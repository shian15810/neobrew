use std::{collections::HashMap, ffi::OsString, path::PathBuf, process::Command};

use clap::{Parser, Subcommand};
use figment::{
    Figment,
    providers::{Env, Serialized},
};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(bin_name = "nbrew", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Install {
        #[arg(value_name = "FORMULA|CASK")]
        packages: Vec<String>,
    },

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
        Commands::Install { packages } => println!("Install packages: {packages:?}"),

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
