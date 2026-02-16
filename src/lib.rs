use std::sync::Arc;

use clap::Parser;

use self::{commands::Cli, context::Context};

mod commands;
mod context;
mod package;
mod pipeline;
mod registries;

pub async fn run() -> proc_exit::ExitResult {
    let cli = Cli::parse();

    let context = Context::new();

    cli.command.run(Arc::new(context)).await
}
