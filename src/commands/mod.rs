use std::{ffi::OsString, sync::Arc};

use anyhow::Result;
use clap::{
    Args,
    ColorChoice,
    Parser,
    Subcommand,
    ValueEnum as _,
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
use crate::{context::Context, registry::ResolutionStrategy};

mod install;
mod uninstall;

#[derive(Parser)]
#[command(
    name = crate_name!(),
    bin_name = env!("CARGO_PKG_METADATA_NEOBREW_BIN_NAME"),
    display_name = env!("CARGO_PKG_METADATA_NEOBREW_DISPLAY_NAME"),
    author = crate_authors!(),
    about = crate_description!().split_once(" - ").map(|(head, _)| head),
    long_about = crate_description!(),
    version = crate_version!(),
    long_version = {
        let version_info = rustc_tools_util::get_version_info!();
        let version_info = version_info.to_string();
        let version_info = version_info.leak();

        version_info
            .split_once(' ')
            .map(|(_, long_version)| long_version)
    },
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    verbosity: Verbosity,

    #[arg(
        long,
        global = true,
        value_name = "WHEN",
        num_args = 0..=1,
        require_equals = true,
        default_value_t = ColorChoice::Auto,
        default_missing_value = {
            let color_choice = ColorChoice::Always.to_possible_value();

            color_choice.map(|color_choice| {
                let color = color_choice.get_name();
                let color = color.to_owned();
                let color: &str = color.leak();

                color
            })
        },
    )]
    color: ColorChoice,
}

impl Cli {
    pub(super) async fn run(self, context: Context) -> proc_exit::ExitResult {
        self.command.run(context).await?;

        proc_exit::Code::SUCCESS.ok()
    }
}

#[derive(Subcommand)]
enum Commands {
    #[command(flatten)]
    Internal(Internal),

    #[command(external_subcommand)]
    External(Vec<OsString>),
}

impl Commands {
    async fn run(self, context: Context) -> proc_exit::ExitResult {
        match self {
            Self::Internal(internal) => {
                let context = Arc::new(context);

                let result = internal.run_concurrent(context).await;

                result
                    .with_code(proc_exit::sysexits::SOFTWARE_ERR)
                    .with_code(proc_exit::sysexits::SOFTWARE_ERR)?;

                proc_exit::Code::SUCCESS.ok()
            },

            Self::External(args) => {
                let mut cmd = Command::new("brew");

                cmd.args(args)
                    .env("HOMEBREW_NO_ANALYTICS", "1")
                    .env("HOMEBREW_NO_ENV_HINTS", "1");

                match context.config.verbosity_filter {
                    VerbosityFilter::Debug => {
                        cmd.env("HOMEBREW_DEBUG", "1");
                    },
                    VerbosityFilter::Info => {
                        cmd.env("HOMEBREW_VERBOSE", "1");
                    },
                    _ => {},
                }

                #[expect(clippy::match_wildcard_for_single_variants)]
                match context.config.color_choice {
                    ColorChoice::Never => {
                        cmd.env("HOMEBREW_NO_COLOR", "1");
                    },
                    ColorChoice::Always => {
                        cmd.env("HOMEBREW_COLOR", "1");
                    },
                    _ => {},
                }

                let result = cmd.status().await;

                let status = result.to_sysexits()?;

                proc_exit::Code::from_status(status).ok()?;

                proc_exit::Code::SUCCESS.ok()
            },
        }
    }
}

#[derive(Subcommand)]
#[enum_dispatch]
enum Internal {
    Install(Install),
    Uninstall(Uninstall),
}

#[enum_dispatch(Internal)]
trait Runner {
    async fn run_concurrent(self, context: Arc<Context>) -> Result<()>;
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
