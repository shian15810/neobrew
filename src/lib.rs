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

use std::sync::Arc;

use console_subscriber as _;
use tracing_subscriber as _;

pub use self::{commands::Cli, context::Context};

mod commands;
mod context;
mod package;
mod pipeline;
mod registries;

#[allow(clippy::missing_errors_doc)]
pub async fn run(cli: Cli, context: Context) -> proc_exit::ExitResult {
    let context = Arc::new(context);

    cli.command.run(context).await?;

    proc_exit::Code::SUCCESS.ok()
}
