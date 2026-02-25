use cfg_if::cfg_if;
use clap::Parser;
use neobrew::Cli;
use tokio::signal;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    init_tracing(&cli);

    let result = tokio::select! {
        biased;

        _ = signal::ctrl_c() => proc_exit::bash::SIGINT.ok(),

        result = neobrew::run(cli) => result,
    };

    proc_exit::exit(result);
}

fn init_tracing(cli: &Cli) {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(cli.verbosity.tracing_level_filter().into())
        .from_env_lossy();

    let filtered_layer = tracing_subscriber::fmt::layer().with_filter(filter);

    cfg_if! {
        if #[cfg(debug_assertions)] {
            let console_layer = console_subscriber::spawn();

            tracing_subscriber::registry()
                .with(console_layer)
                .with(filtered_layer)
                .init();
        } else {
            tracing_subscriber::registry().with(filtered_layer).init();
        }
    }
}
