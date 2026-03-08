#![cfg_attr(
    debug_assertions,
    feature(
        multiple_supertrait_upcastable,
        must_not_suspend,
        non_exhaustive_omitted_patterns_lint,
        strict_provenance_lints,
        supertrait_item_shadowing,
        unqualified_local_imports,
    )
)]
#![cfg_attr(
    debug_assertions,
    warn(
        fuzzy_provenance_casts,
        lossy_provenance_casts,
        multiple_supertrait_upcastable,
        must_not_suspend,
        non_exhaustive_omitted_patterns,
        resolving_to_items_shadowing_supertrait_items,
        shadowing_supertrait_items,
        unqualified_local_imports,
    )
)]
#![allow(unused_crate_dependencies)]

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches};
use clap_verbosity_flag::VerbosityFilter;
use neobrew::{Cli, Context};
use proc_exit::prelude::*;
use tokio::{
    signal,
    task::{self, JoinHandle},
};
use tracing_subscriber::{filter::LevelFilter, prelude::*};

#[tokio::main]
async fn main() -> proc_exit::ExitResult {
    let matches = Cli::command().get_matches();

    let context = Context::new(&matches).with_code(proc_exit::sysexits::CONFIG_ERR)?;

    init_tracing(context.config.verbosity_filter);

    let handle: JoinHandle<Result<()>> = task::spawn(async move {
        signal::ctrl_c().await?;

        Ok(())
    });

    let cli = Cli::from_arg_matches(&matches).with_code(proc_exit::sysexits::USAGE_ERR)?;

    #[allow(clippy::disallowed_macros)]
    let result = tokio::select! {
        biased;

        res = handle => {
            res.with_code(proc_exit::sysexits::SOFTWARE_ERR)?
                .with_code(proc_exit::sysexits::SOFTWARE_ERR)?;

            proc_exit::bash::SIGINT.ok()
        },

        res = neobrew::run(cli, context) => res,
    };

    result?;

    proc_exit::Code::SUCCESS.ok()
}

fn init_tracing(verbosity_filter: VerbosityFilter) {
    let registry = tracing_subscriber::registry();

    #[cfg(debug_assertions)]
    let registry = {
        let console_layer = console_subscriber::spawn();

        registry.with(console_layer)
    };

    let level_filter = LevelFilter::from(verbosity_filter);

    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(level_filter.into())
        .from_env_lossy();

    let filtered_layer = tracing_subscriber::fmt::layer().with_filter(filter);

    registry.with(filtered_layer).init();
}
