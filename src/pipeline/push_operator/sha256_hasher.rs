use base16ct::HexDisplay;
use bytes::Bytes;
use sha2::{Digest as _, Sha256};

use super::PushOperator;

pub(crate) struct Sha256Hasher {
    digest: Sha256,
}

impl Sha256Hasher {
    pub(crate) fn new() -> Self {
        Self {
            digest: Sha256::new(),
        }
    }
}

impl PushOperator for Sha256Hasher {
    type Item = Bytes;
    type Output = String;

    #[expect(clippy::unused_async_trait_impl)]
    async fn feed(&mut self, chunk: Self::Item) -> anyhow::Result<()> {
        self.digest.update(chunk);

        Ok(())
    }

    #[expect(clippy::unused_async_trait_impl)]
    async fn flush(self) -> anyhow::Result<Self::Output> {
        let hashed_sha256 = self.digest.finalize();
        let hashed_sha256 = HexDisplay(&hashed_sha256);
        let hashed_sha256 = format!("{hashed_sha256:x}");

        Ok(hashed_sha256)
    }
}
