use std::{ffi::OsString, io, process::ExitStatus, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use clap::{Args, Parser, Subcommand};
use enum_dispatch::enum_dispatch;
use tokio::process::Command;

use self::{install::Install, uninstall::Uninstall};
use crate::{context::Context, package::ResolutionStrategy};

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
    async fn run(&self, context: Arc<Context>) -> Result<()>;
}

#[derive(Args)]
struct Resolution {
    #[arg(long, alias = "formulae", conflicts_with = "cask")]
    formula: bool,

    #[arg(long, alias = "casks", conflicts_with = "formula")]
    cask: bool,
}

impl Resolution {
    fn strategy(&self) -> ResolutionStrategy {
        match (self.formula, self.cask) {
            (true, _) => ResolutionStrategy::FormulaOnly,
            (_, true) => ResolutionStrategy::CaskOnly,
            _ => ResolutionStrategy::Both,
        }
    }
}

pub async fn run_external(args: &[OsString]) -> io::Result<ExitStatus> {
    Command::new("brew")
        .args(args)
        .env("HOMEBREW_NO_ANALYTICS", "1")
        .env("HOMEBREW_NO_AUTOREMOVE", "1")
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .env("HOMEBREW_NO_ENV_HINTS", "1")
        .env("HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK", "1")
        .env("HOMEBREW_NO_INSTALL_CLEANUP", "1")
        .env("HOMEBREW_NO_INSTALL_UPGRADE", "1")
        .status()
        .await
}
