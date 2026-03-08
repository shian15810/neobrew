use std::{ffi::OsString, sync::Arc};

use anyhow::Result;
use clap::{
    Args,
    ColorChoice,
    Parser,
    Subcommand,
    ValueEnum,
    crate_authors,
    crate_description,
    crate_name,
    crate_version,
};
use clap_verbosity_flag::{Verbosity, VerbosityFilter};
use enum_dispatch::enum_dispatch;
use proc_exit::prelude::*;
use tokio::process::Command;

use self::{install::Install, uninstall::Uninstall};
use crate::{context::Context, registries::ResolutionStrategy};

mod install;
mod uninstall;

#[derive(Parser, Debug)]
#[command(
    name = crate_name!(),
    bin_name = env!("CARGO_PKG_METADATA_NEOBREW_BIN_NAME"),
    display_name = env!("CARGO_PKG_METADATA_NEOBREW_DISPLAY_NAME"),
    author = crate_authors!(),
    about = crate_description!().split_once(" - ").map(|(head, _)| head),
    long_about = crate_description!(),
    version = crate_version!(),
    long_version = rustc_tools_util::get_version_info!()
        .to_string()
        .leak()
        .split_once(' ')
        .map(|(_, tail)| tail),
)]
pub struct Cli {
    #[command(subcommand)]
    pub(super) command: Commands,

    #[command(flatten)]
    pub verbosity: Verbosity,

    #[arg(
        long,
        global = true,
        value_name = "WHEN",
        num_args = 0..=1,
        require_equals = true,
        default_value_t = ColorChoice::Auto,
        default_missing_value = ColorChoice::Always
            .to_possible_value()
            .map(|val| -> &str { val.get_name().to_owned().leak() }),
    )]
    color: ColorChoice,
}

#[derive(Subcommand, Debug)]
pub(super) enum Commands {
    #[command(flatten)]
    Internal(Internal),

    #[command(external_subcommand)]
    External(Vec<OsString>),
}

impl Commands {
    pub(super) async fn run(self, context: Arc<Context>) -> proc_exit::ExitResult {
        match self {
            Self::Internal(internal) => {
                let res = internal.run(context).await;

                res.with_code(proc_exit::sysexits::SOFTWARE_ERR)?;

                proc_exit::Code::SUCCESS.ok()
            },

            Self::External(args) => {
                let mut cmd = Command::new("brew");

                let _ = cmd.args(args);

                match context.config.verbosity_filter {
                    VerbosityFilter::Debug => {
                        let _ = cmd.env("HOMEBREW_DEBUG", "1");
                    },
                    VerbosityFilter::Info => {
                        let _ = cmd.env("HOMEBREW_VERBOSE", "1");
                    },
                    _ => {},
                }

                #[allow(clippy::match_wildcard_for_single_variants)]
                match context.config.color_choice {
                    ColorChoice::Never => {
                        let _ = cmd.env("HOMEBREW_NO_COLOR", "1");
                    },
                    ColorChoice::Always => {
                        let _ = cmd.env("HOMEBREW_COLOR", "1");
                    },
                    _ => {},
                }

                let _ = cmd
                    .env("HOMEBREW_NO_ANALYTICS", "1")
                    .env("HOMEBREW_NO_AUTOREMOVE", "1")
                    .env("HOMEBREW_NO_AUTO_UPDATE", "1")
                    .env("HOMEBREW_NO_ENV_HINTS", "1")
                    .env("HOMEBREW_NO_INSECURE_REDIRECT", "1")
                    .env("HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK", "1")
                    .env("HOMEBREW_NO_INSTALL_CLEANUP", "1")
                    .env("HOMEBREW_NO_INSTALL_UPGRADE", "1");

                let res = cmd.status().await;

                let status = res.to_sysexits()?;

                proc_exit::Code::from_status(status).ok()?;

                proc_exit::Code::SUCCESS.ok()
            },
        }
    }
}

#[derive(Subcommand, Debug)]
#[enum_dispatch]
pub(super) enum Internal {
    Install(Install),
    Uninstall(Uninstall),
}

#[enum_dispatch(Internal)]
trait Runner {
    async fn run(self, context: Arc<Context>) -> Result<()>;
}

#[derive(Args, Debug)]
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
