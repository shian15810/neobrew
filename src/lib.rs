#![feature(
    multiple_supertrait_upcastable,
    must_not_suspend,
    non_exhaustive_omitted_patterns_lint,
    strict_provenance_lints,
    supertrait_item_shadowing,
    unqualified_local_imports
)]

use std::sync::Arc;

use clap::CommandFactory;
use proc_exit::prelude::*;

pub use self::commands::Cli;
use self::context::Context;

mod commands;
mod context;
mod package;
mod pipeline;
mod registries;

#[allow(clippy::missing_errors_doc)]
pub async fn run(cli: Cli) -> proc_exit::ExitResult {
    let matches = Cli::command().get_matches();

    let context = Context::new(&matches).with_code(proc_exit::sysexits::CONFIG_ERR)?;
    let context = Arc::new(context);

    cli.command.run(context).await?;

    proc_exit::Code::SUCCESS.ok()
}
