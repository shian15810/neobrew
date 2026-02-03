use std::{collections::HashMap, ffi::OsString, io, process::ExitStatus};

use anyhow::Result;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use enum_dispatch::enum_dispatch;
use tokio::process::Command;

use crate::{
    commands::{install::Install, uninstall::Uninstall},
    context::config::Config,
};

mod install;
mod uninstall;

#[derive(Parser)]
#[command(bin_name = "nbrew", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(flatten)]
    Internal(Internal),

    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[enum_dispatch]
#[derive(Subcommand)]
pub enum Internal {
    Install(Install),
    Uninstall(Uninstall),
}

#[async_trait]
#[enum_dispatch(Internal)]
pub trait Runner {
    async fn run(&self, config: &Config) -> Result<()>;
}

pub async fn run_external(args: &Vec<OsString>) -> io::Result<ExitStatus> {
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
