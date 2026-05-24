use anyhow::Result;
use base16ct::HexDisplay;
use bytes::Bytes;
use sha2::{Digest as _, Sha256};

use super::PushOperator;

pub(crate) struct Hasher {
    sha256: Sha256,
}

impl Hasher {
    pub(crate) fn new() -> Self {
        Self {
            sha256: Sha256::new(),
        }
    }
}

impl PushOperator for Hasher {
    type Item = Bytes;
    type Output = String;

    async fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        self.sha256.update(chunk);

        Ok(())
    }

    async fn flush(self) -> Result<Self::Output> {
        let hashed_sha256 = self.sha256.finalize();
        let hashed_sha256 = HexDisplay(&hashed_sha256);
        let hashed_sha256 = format!("{hashed_sha256:x}");

        Ok(hashed_sha256)
    }
}
