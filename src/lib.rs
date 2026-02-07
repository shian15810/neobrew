use std::sync::Arc;

use clap::Parser;
use proc_exit::prelude::*;

use self::{
    commands::{Cli, Commands, Runner, run_external},
    context::Context,
};

mod commands;
mod context;
mod package;

pub async fn run() -> proc_exit::ExitResult {
    let cli = Cli::parse();
    let context = Context::new();

    match &cli.command {
        Commands::Internal(cmd) => cmd
            .run(Arc::new(context))
            .await
            .with_code(proc_exit::sysexits::SOFTWARE_ERR),

        Commands::External(args) => {
            let exit_status = run_external(args).await.to_sysexits()?;

            proc_exit::Code::from_status(exit_status).ok()?;
            proc_exit::Code::SUCCESS.ok()
        },
    }
}
