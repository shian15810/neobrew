use anyhow::Result;
use bytes::Bytes;
use sha2::{Digest, Sha256};

use super::Operator;

pub struct Hasher {
    inner: Sha256,
}

impl Hasher {
    pub fn new() -> Self {
        Self {
            inner: Sha256::new(),
        }
    }
}

impl Operator for Hasher {
    fn send(&mut self, chunk: Bytes) -> Result<()> {
        self.inner.update(chunk);

        Ok(())
    }

    fn apply(self) -> Result<()> {
        let _result = self.inner.finalize();

        Ok(())
    }
}
