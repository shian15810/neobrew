use std::path::PathBuf;

use anyhow::Result;
use apple_codesign::{SigningSettings, UnifiedSigner};
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

pub(crate) struct Codesign;

impl Codesign {
    pub(crate) async fn in_place(path: PathBuf) -> Result<()> {
        let handle = task::spawn_blocking(move || {
            let settings = SigningSettings::default();

            let signer = UnifiedSigner::new(settings);

            signer.sign_path_in_place(path)?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }
}
