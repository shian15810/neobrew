use std::{collections::HashMap, ffi::OsString, process::Command};

use clap::{Parser, Subcommand};

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

fn main() {
    let cli = Cli::parse();

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
