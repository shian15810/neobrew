use anyhow::Result;
use bytes::Bytes;
use sha2::{Digest, Sha256};

use super::TeeOperator;

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

impl TeeOperator<Bytes, String> for Hasher {
    fn feed(&mut self, chunk: Bytes) -> Result<()> {
        self.inner.update(chunk);

        Ok(())
    }

    fn flush(self) -> Result<String> {
        let result = self.inner.finalize();

        let output = format!("{result:x}");

        Ok(output)
    }
}
