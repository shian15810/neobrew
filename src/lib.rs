use std::sync::Arc;

use proc_exit::prelude::*;

pub use self::commands::Cli;
use self::context::Context;

mod commands;
mod context;
mod package;
mod pipeline;
mod registries;

pub async fn run(cli: Cli) -> proc_exit::ExitResult {
    let context = Context::new();
    let context = Arc::new(context);

    context
        .homebrew_config()
        .with_code(proc_exit::sysexits::SOFTWARE_ERR)?
        .ensure_default_prefix()
        .with_code(proc_exit::sysexits::CONFIG_ERR)?;

    cli.command.run(context).await
}
