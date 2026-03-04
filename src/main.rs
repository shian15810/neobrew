#![feature(
    multiple_supertrait_upcastable,
    must_not_suspend,
    non_exhaustive_omitted_patterns_lint,
    strict_provenance_lints,
    supertrait_item_shadowing,
    unqualified_local_imports
)]

use anyhow::Result;
use clap::Parser;
use neobrew::Cli;
use proc_exit::prelude::*;
use tokio::{
    signal,
    task::{self, JoinHandle},
};
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> proc_exit::ExitResult {
    let cli = Cli::parse();

    init_tracing(&cli);

    let handle: JoinHandle<Result<()>> = task::spawn(async move {
        signal::ctrl_c().await?;

        Ok(())
    });

    let result = tokio::select! {
        biased;

        res = handle => {
            res.with_code(proc_exit::sysexits::SOFTWARE_ERR)?
                .with_code(proc_exit::sysexits::SOFTWARE_ERR)?;

            proc_exit::bash::SIGINT.ok()
        },

        res = neobrew::run(cli) => res,
    };

    result?;

    proc_exit::Code::SUCCESS.ok()
}

fn init_tracing(cli: &Cli) {
    let registry = tracing_subscriber::registry();

    #[cfg(debug_assertions)]
    let registry = {
        let console_layer = console_subscriber::spawn();

        registry.with(console_layer)
    };

    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(cli.verbosity.tracing_level_filter().into())
        .from_env_lossy();

    let filtered_layer = tracing_subscriber::fmt::layer().with_filter(filter);

    registry.with(filtered_layer).init();
}
