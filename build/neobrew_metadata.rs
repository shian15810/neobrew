use std::array;

use anyhow::Result;
use cargo_toml::Manifest;
use serde::Deserialize;

#[derive(Deserialize)]
struct Metadata {
    neobrew: NeobrewMetadata,
}

#[derive(Deserialize)]
pub(super) struct NeobrewMetadata {
    bin_name: String,
    display_name: String,
}

impl NeobrewMetadata {
    const RERUN_IF_CHANGED: &[&str] = &["Cargo.toml"];
    const RERUN_IF_ENV_CHANGED: &[&str] = &[];

    const ENV_PREFIX: &str = "CARGO_PKG_METADATA_NEOBREW_";

    pub(super) fn setup() -> Result<()> {
        Self::rerun_if_changed();
        Self::rerun_if_env_changed();

        Self::set_rustc_env()?;

        Ok(())
    }

    fn rerun_if_changed() {
        for &path in Self::RERUN_IF_CHANGED {
            println!("cargo::rerun-if-changed={path}");
        }
    }

    fn rerun_if_env_changed() {
        for &var in Self::RERUN_IF_ENV_CHANGED {
            println!("cargo::rerun-if-env-changed={var}");
        }
    }

    fn set_rustc_env() -> Result<()> {
        let cargo_toml_path = env!("CARGO_MANIFEST_PATH");

        let manifest = Manifest::<Metadata>::from_path_with_metadata(cargo_toml_path)?;

        let metadata = &manifest.package().metadata;

        let Some(metadata) = metadata else {
            return Ok(());
        };

        let neobrew_metadata = &metadata.neobrew;

        for (key, value) in neobrew_metadata {
            let var = format!("{}{}", Self::ENV_PREFIX, key.to_uppercase());

            println!("cargo::rustc-env={var}={value}");
        }

        Ok(())
    }
}

impl<'a> IntoIterator for &'a NeobrewMetadata {
    type Item = (&'static str, &'a str);
    type IntoIter = array::IntoIter<Self::Item, 2>;

    fn into_iter(self) -> Self::IntoIter {
        let this = [
            ("bin_name", &self.bin_name),
            ("display_name", &self.display_name),
        ];
        let this = this.map(|(key, val)| (key, val.as_str()));

        this.into_iter()
    }
}
