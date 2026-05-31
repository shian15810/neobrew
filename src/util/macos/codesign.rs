use std::path::Path;

use apple_codesign::{SigningSettings, UnifiedSigner};
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

pub(crate) struct Codesign;

impl Codesign {
    pub(crate) async fn in_place(path: &Path) -> anyhow::Result<()> {
        let path = path.to_owned();

        let handle = task::spawn_blocking(|| {
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
