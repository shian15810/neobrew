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
        let output = format!("{:x}", HexDisplay(&self.inner.finalize()));

        Ok(output)
    }
}
