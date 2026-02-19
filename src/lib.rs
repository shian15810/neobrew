use std::sync::Arc;

use clap::Parser;
use proc_exit::prelude::*;
use tokio::signal;

use self::{commands::Cli, context::Context};

mod commands;
mod context;
mod package;
mod pipeline;
mod registries;

pub async fn run() -> proc_exit::ExitResult {
    let cli = Cli::parse();

    let context = Context::new();

    context
        .homebrew_config()
        .with_code(proc_exit::sysexits::SOFTWARE_ERR)?
        .ensure_default_prefix()
        .with_code(proc_exit::sysexits::CONFIG_ERR)?;

    tokio::select! {
        biased;

        _ = signal::ctrl_c() => proc_exit::bash::SIGINT.ok(),

        exit_result = cli.command.run(Arc::new(context)) => exit_result,
    }
}
