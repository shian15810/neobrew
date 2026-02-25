use anyhow::{Result, anyhow};
use cfg_if::cfg_if;
use figment::util::bool_from_str_or_int;
use indoc::indoc;
use serde::{Deserialize, Serialize};

use super::Config;

#[derive(Serialize, Deserialize)]
pub struct HomebrewConfig {
    prefix: String,

    #[serde(deserialize_with = "bool_from_str_or_int")]
    verbose: bool,

    #[serde(deserialize_with = "bool_from_str_or_int")]
    color: bool,

    #[serde(deserialize_with = "bool_from_str_or_int")]
    no_color: bool,
}

impl Config for HomebrewConfig {
    const ENV_PREFIX: &str = "HOMEBREW_";
}

impl Default for HomebrewConfig {
    fn default() -> Self {
        Self {
            prefix: Self::DEFAULT_PREFIX.to_owned(),

            verbose: false,

            color: false,
            no_color: false,
        }
    }
}

impl HomebrewConfig {
    cfg_if! {
        if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
            const DEFAULT_PREFIX: &str = "/opt/homebrew";
        } else if #[cfg(all(target_os = "macos", target_arch = "x86_64"))] {
            const DEFAULT_PREFIX: &str = "/usr/local";
        } else {
            compile_error!("This crate only supports macOS (aarch64 and x86_64).");
        }
    }

    pub fn ensure_default_prefix(&self) -> Result<()> {
        if self.prefix == Self::DEFAULT_PREFIX {
            return Ok(());
        }

        let err = anyhow!(
            indoc! {r#"
                Unsupported `HOMEBREW_PREFIX`: "{}"

                Neobrew requires the default prefix to use pre-compiled bottles and casks:

                  • Apple Silicon  →  "/opt/homebrew"
                  • Intel x86_64   →  "/usr/local"

                The default prefix is essential for Neobrew's high-performance guarantees,
                seamless developer experience, and smooth interoperability with your local
                Homebrew installation — so you can use `nbrew` and `brew` interchangeably.

                See https://docs.brew.sh/Installation"#},
            self.prefix,
        );

        Err(err)
    }
}
