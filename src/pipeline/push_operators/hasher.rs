use anyhow::Result;
use bytes::Bytes;
use sha2::{Digest, Sha256, digest};

use super::PushOperator;

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

impl PushOperator for Hasher {
    type Item = Bytes;
    type Output = digest::Output<Sha256>;

    fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        self.inner.update(chunk);

        Ok(())
    }

    fn flush(self) -> Result<Self::Output> {
        let output = self.inner.finalize();

        Ok(output)
    }
}
