use anyhow::Result;
use base16ct::HexDisplay;
use bytes::Bytes;
use sha2::{Digest as _, Sha256};

use super::PushOperator;

pub(crate) struct Hasher {
    inner: Sha256,
}

impl Hasher {
    pub(crate) fn new() -> Self {
        Self {
            inner: Sha256::new(),
        }
    }
}

impl PushOperator for Hasher {
    type Item = Bytes;
    type Output = String;

    fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        self.inner.update(chunk);

        Ok(())
    }

    fn flush(self) -> Result<Self::Output> {
        let fetch_hash = self.inner.finalize();
        let fetch_hash = HexDisplay(&fetch_hash);
        let fetch_hash = format!("{fetch_hash:x}");

        Ok(fetch_hash)
    }
}
