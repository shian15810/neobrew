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

use clap::{ArgMatches, FromArgMatches};
use proc_exit::prelude::*;

pub use self::{commands::Cli, context::Context};

mod commands;
mod context;
mod package;
mod pipeline;
mod registry;

use console_subscriber as _;
use tracing_subscriber as _;

#[allow(clippy::missing_errors_doc)]
pub async fn run(matches: &ArgMatches, context: Context) -> proc_exit::ExitResult {
    let cli = Cli::from_arg_matches(matches);
    let cli = cli.with_code(proc_exit::sysexits::USAGE_ERR)?;

    cli.run(context).await?;

    proc_exit::Code::SUCCESS.ok()
}

use oci_client as _;
use os_info as _;
