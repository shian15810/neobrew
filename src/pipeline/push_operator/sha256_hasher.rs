use std::sync::Arc;

use anyhow::anyhow;
use base16ct::HexDisplay;
use bytes::Bytes;
use sha2::{Digest as _, Sha256};

use super::{super::channels::PipelineChannels as Channels, PushOperator};
use crate::context::Context;

pub(crate) struct Sha256Hasher {
    digest: Sha256,

    expected: String,
}

impl Sha256Hasher {
    pub(crate) fn new(expected: String) -> Self {
        Self {
            digest: Sha256::new(),

            expected,
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

    async fn flush(
        self,
        channels: Arc<Channels>,
        _context: Arc<Context>,
    ) -> anyhow::Result<Self::Output> {
        let digest = self.digest.clone();

        let hashed = digest.finalize();
        let hashed = HexDisplay(&hashed);
        let hashed = format!("{hashed:x}");

        let is_verified = hashed == self.expected;
        let is_verified = Some(is_verified);

        channels.is_verified_tx.send(is_verified)?;

        if matches!(is_verified, Some(false)) {
            self.cleanup()?;

            let err = anyhow!("Hasher failed due to SHA-256 mismatch");

            return Err(err);
        }

        self.persist().await?;

        let output = hashed;

        Ok(output)
    }

    fn cleanup(self) -> anyhow::Result<()> {
        Ok(())
    }

    #[expect(clippy::unused_async_trait_impl)]
    async fn persist(self) -> anyhow::Result<()> {
        Ok(())
    }
}
