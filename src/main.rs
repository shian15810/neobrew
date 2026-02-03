use std::sync::Arc;

use clap::Parser;
use neobrew::{
    commands::{Cli, Commands, Runner, run_external},
    context::Context,
};
use proc_exit::prelude::*;

#[tokio::main]
async fn main() {
    let result = run().await;

    proc_exit::exit(result);
}

async fn run() -> proc_exit::ExitResult {
    let cli = Cli::parse();
    let context = Arc::new(Context::new());

    match &cli.command {
        Commands::Internal(cmd) => cmd
            .run(context)
            .await
            .with_code(proc_exit::sysexits::SOFTWARE_ERR),

        Commands::External(args) => {
            let exit_status = run_external(args).await.to_sysexits()?;

            proc_exit::Code::from_status(exit_status).ok()?;
            proc_exit::Code::SUCCESS.ok()
        },
    }
}
