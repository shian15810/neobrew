use anyhow::Error;
use clap::Parser;
use neobrew::Cli;
use proc_exit::prelude::*;
use tokio::{signal, task};
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    init_tracing(&cli);

    let handle = task::spawn(async move {
        signal::ctrl_c().await?;

        Ok::<_, Error>(())
    });

    let result = tokio::select! {
        biased;

        res = handle => match res.map_err(Error::from).flatten() {
            Ok(()) => proc_exit::bash::SIGINT.ok(),
            Err(e) => Err(e).with_code(proc_exit::sysexits::SOFTWARE_ERR),
        },

        res = neobrew::run(cli) => res,
    };

    proc_exit::exit(result);
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
