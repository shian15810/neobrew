use std::{ffi::OsString, sync::Arc};

use anyhow::Result;
use clap::{Args, ColorChoice, Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::Verbosity;
use enum_dispatch::enum_dispatch;
use proc_exit::prelude::*;
use tokio::process::Command;

use self::{install::Install, uninstall::Uninstall};
use crate::{context::Context, registries::ResolutionStrategy};

mod install;
mod uninstall;

#[derive(Parser)]
#[command(display_name = "Neobrew", bin_name = "nbrew", version, author)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub verbosity: Verbosity,

    #[arg(
        long,
        global = true,
        value_name = "WHEN",
        num_args = 0..=1,
        require_equals = true,
        default_value_t = ColorChoice::Auto,
        default_missing_value = &*ColorChoice::Always
            .to_possible_value()
            .unwrap()
            .get_name()
            .to_owned()
            .leak(),
    )]
    color: ColorChoice,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(flatten)]
    Internal(Internal),

    #[command(external_subcommand)]
    External(Vec<OsString>),
}

impl Commands {
    pub async fn run(self, context: Arc<Context>) -> proc_exit::ExitResult {
        match self {
            Self::Internal(internal) => internal
                .run(context)
                .await
                .with_code(proc_exit::sysexits::SOFTWARE_ERR),

            Self::External(args) => {
                let mut cmd = Command::new("brew");

                cmd.args(args)
                    .env("HOMEBREW_VERBOSE", "0")
                    .env("HOMEBREW_COLOR", "0")
                    .env("HOMEBREW_NO_COLOR", "0")
                    .env("HOMEBREW_NO_ANALYTICS", "1")
                    .env("HOMEBREW_NO_AUTOREMOVE", "1")
                    .env("HOMEBREW_NO_AUTO_UPDATE", "1")
                    .env("HOMEBREW_NO_ENV_HINTS", "1")
                    .env("HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK", "1")
                    .env("HOMEBREW_NO_INSTALL_CLEANUP", "1")
                    .env("HOMEBREW_NO_INSTALL_UPGRADE", "1");

                let status = cmd.status().await.to_sysexits()?;

                proc_exit::Code::from_status(status).ok()?;

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

#[enum_dispatch(Internal)]
trait Runner {
    async fn run(self, context: Arc<Context>) -> Result<()>;
}

#[derive(Args)]
struct Resolution {
    #[arg(long, alias = "formulae", conflicts_with = "cask")]
    formula: bool,

    #[arg(long, alias = "casks", conflicts_with = "formula")]
    cask: bool,
}

impl Resolution {
    fn strategy(self) -> ResolutionStrategy {
        match (self.formula, self.cask) {
            (true, _) => ResolutionStrategy::FormulaOnly,
            (_, true) => ResolutionStrategy::CaskOnly,
            _ => ResolutionStrategy::Both,
        }
    }
}
