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
            build_rs::output::rerun_if_changed(path);
        }
    }

    fn rerun_if_env_changed() {
        for &key in Self::RERUN_IF_ENV_CHANGED {
            build_rs::output::rerun_if_env_changed(key);
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
            let key = format!("{}{}", Self::ENV_PREFIX, key.to_uppercase());

            build_rs::output::rustc_env(&key, value);
        }

        Ok(())
    }
}

impl<'a> IntoIterator for &'a NeobrewMetadata {
    type Item = (&'static str, &'a str);
    type IntoIter = array::IntoIter<Self::Item, 2>;

    fn into_iter(self) -> Self::IntoIter {
        let collection = [
            ("bin_name", &self.bin_name),
            ("display_name", &self.display_name),
        ];
        let collection = collection.map(|(key, val)| (key, val.as_str()));

        collection.into_iter()
    }
}
