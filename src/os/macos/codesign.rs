use std::path::Path;

use anyhow::Result;
use apple_codesign::{SigningSettings, UnifiedSigner};

pub(crate) struct Codesign;

impl Codesign {
    pub(crate) fn sign_in_place(path: impl AsRef<Path>) -> Result<()> {
        let settings = SigningSettings::default();

        let signer = UnifiedSigner::new(settings);

        signer.sign_path_in_place(path)?;

        Ok(())
    }
}
