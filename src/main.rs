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
        dead_code_pub_in_binary,
        fuzzy_provenance_casts,
        linker_info,
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

use clap::CommandFactory as _;
use clap_verbosity_flag::VerbosityFilter;
use neobrew::{command::Cli, context::Context};
use proc_exit::WithCodeResultExt as _;
use tokio::{signal, task};
use tracing_subscriber::{
    EnvFilter,
    filter::{Directive, LevelFilter},
    fmt,
    layer::{Layer as _, SubscriberExt as _},
    util::SubscriberInitExt as _,
};

#[tokio::main]
async fn main() -> proc_exit::ExitResult {
    let matches = Cli::command().get_matches();

    let context = Context::load(&matches)?;

    init_tracing(*context.config().verbosity_filter());

    let handle = task::spawn(async {
        signal::ctrl_c().await?;

        anyhow::Ok(())
    });

    #[expect(clippy::disallowed_macros)]
    let result = tokio::select! {
        biased;

        result = handle => {
            result
                .with_code(proc_exit::sysexits::SOFTWARE_ERR)?
                .with_code(proc_exit::sysexits::OS_ERR)?;

            proc_exit::bash::SIGINT.ok()
        },

        result = neobrew::run(&matches, context) => result,
    };

    result?;

    proc_exit::Code::SUCCESS.ok()
}

fn init_tracing(verbosity_filter: VerbosityFilter) {
    let registry = tracing_subscriber::registry();

    #[cfg(all(debug_assertions, not(test)))]
    let registry = {
        let console_layer = console_subscriber::spawn();

        registry.with(console_layer)
    };

    let level_filter = LevelFilter::from(verbosity_filter);

    let default_directive = Directive::from(level_filter);

    let filter = EnvFilter::builder()
        .with_default_directive(default_directive)
        .from_env_lossy();

    let filtered_layer = fmt::layer().with_filter(filter);

    registry.with(filtered_layer).init();
}

#[cfg(not(all(debug_assertions, not(test))))]
use console_subscriber as _;
