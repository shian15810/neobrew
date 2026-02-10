use std::{ffi::OsString, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use clap::{Args, Parser, Subcommand};
use enum_dispatch::enum_dispatch;
use proc_exit::prelude::*;
use tokio::process::Command;

use self::{install::Install, uninstall::Uninstall};
use crate::{context::Context, registries::ResolutionStrategy};

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

impl Commands {
    pub async fn run(&self, context: Arc<Context>) -> proc_exit::ExitResult {
        match self {
            Self::Internal(internal) => internal
                .run(context)
                .await
                .with_code(proc_exit::sysexits::SOFTWARE_ERR),

            Self::External(args) => {
                let exit_status = Command::new("brew")
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
                    .to_sysexits()?;

                proc_exit::Code::from_status(exit_status).ok()?;
                proc_exit::Code::SUCCESS.ok()
            },
        }
    }
}

#[derive(Subcommand)]
#[enum_dispatch]
pub enum Internal {
    Install(Install),
    Uninstall(Uninstall),
}

#[async_trait]
#[enum_dispatch(Internal)]
trait Runner {
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
