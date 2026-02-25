use std::sync::Arc;

use cfg_if::cfg_if;
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

    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(cli.verbosity.tracing_level_filter().into())
        .from_env_lossy();

    cfg_if! {
        if #[cfg(debug_assertions)] {
            use tracing_subscriber::prelude::*;

            let console_layer = console_subscriber::spawn();

            let filtered_layer = tracing_subscriber::fmt::layer().with_filter(filter);

            tracing_subscriber::registry()
                .with(console_layer)
                .with(filtered_layer)
                .init();
        } else {
            tracing_subscriber::fmt().with_env_filter(filter).init();
        }
    }

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
