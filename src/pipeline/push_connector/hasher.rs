use anyhow::anyhow;
use async_trait::async_trait;
use base16ct::HexDisplay;
use bytes::Bytes;
use sha2::{Digest as _, Sha256};

use super::{
    super::state_store::{HashedOutput, Stage},
    PushConnector,
};

pub(crate) struct Hasher {
    sha256_digest: Sha256,

    expected_sha256: String,

    should_run: bool,
}

impl Hasher {
    pub(crate) fn new(expected_sha256: String, should_run: bool) -> Self {
        Self {
            sha256_digest: Sha256::new(),

            expected_sha256,

            should_run,
        }
    }
}

#[async_trait]
impl PushConnector for Hasher {
    type Staging = String;
    type Output = HashedOutput;

    fn should_run(&self) -> bool {
        self.should_run
    }

    async fn on_skip_run(self) -> anyhow::Result<Option<Self::Output>> {
        let output = HashedOutput {
            is_verified: true,

            actual_sha256: self.expected_sha256.clone(),
            expected_sha256: self.expected_sha256,
        };

        Ok(Some(output))
    }

    async fn feed(&mut self, chunk: Bytes) -> anyhow::Result<()> {
        self.sha256_digest.update(chunk);

        Ok(())
    }

    async fn flush(&mut self) -> anyhow::Result<Self::Staging> {
        let sha256_digest = self.sha256_digest.clone();

        let actual_sha256 = sha256_digest.finalize();
        let actual_sha256 = HexDisplay(&actual_sha256);
        let actual_sha256 = format!("{actual_sha256:x}");

        Ok(actual_sha256)
    }

    async fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output> {
        let actual_sha256 = staging;

        let is_verified = actual_sha256 == self.expected_sha256;

        if !is_verified {
            let err = anyhow!("Hasher failed due to SHA-256 mismatch");

            return Err(err);
        }

        let output = HashedOutput {
            is_verified,

            actual_sha256,
            expected_sha256: self.expected_sha256,
        };

        Ok(output)
    }

    fn passed_prefix(&self) -> Option<&'static str> {
        Some("Verified")
    }

    fn failed_prefix(&self) -> Option<&'static str> {
        Some("Mismatched")
    }

    fn passed_stage(&self, _should_run: bool) -> Option<Stage> {
        Some(Stage::Hashed)
    }
}
