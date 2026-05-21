#![cfg_attr(
    debug_assertions,
    feature(const_precise_live_drops, const_trait_impl, iterator_try_collect)
)]
#![cfg_attr(debug_assertions, feature(doc_cfg))]
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
#![cfg_attr(all(debug_assertions, doc), feature(rustdoc_missing_doc_code_examples))]
#![cfg_attr(debug_assertions, allow(rustdoc::missing_doc_code_examples))]
#![doc(test(attr(warn(unused), deny(warnings))))]
#![expect(rustdoc::missing_crate_level_docs)]

pub mod commands;
pub mod context;
mod ext;
mod macros;
mod os;
mod package;
mod pipeline;
mod registry;

use clap::{ArgMatches, FromArgMatches as _};
use proc_exit::prelude::*;

use self::{commands::Cli, context::Context};

#[expect(clippy::missing_errors_doc)]
pub async fn run(matches: &ArgMatches, context: Context) -> proc_exit::ExitResult {
    let cli = Cli::from_arg_matches(matches);
    let cli = cli.with_code(proc_exit::sysexits::USAGE_ERR)?;

    cli.run(context).await?;

    proc_exit::Code::SUCCESS.ok()
}

#[cfg(debug_assertions)]
use visibility as _;
