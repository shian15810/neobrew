use anyhow::anyhow;
use async_trait::async_trait;
use base16ct::HexDisplay;
use bytes::Bytes;
use sha2::{Digest as _, Sha256};

use super::{
    super::state_store::{HashedOutput, Stage},
    PushConnector,
};
use crate::package::prepared::{PreparedPackage, PreparedPackageExt as _, download::Download};

pub(crate) struct Hasher;

#[async_trait]
impl PushConnector for Hasher {
    type State = Sha256;
    type Staging = String;
    type Output = HashedOutput;

    fn should_run(&self, prepared_package: &PreparedPackage<Download>) -> bool {
        let download = prepared_package.download();

        !download.is_verified()
    }

    async fn on_skip_run(
        self,
        prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Option<Self::Output>> {
        let download = prepared_package.download();

        let expected_sha256 = download.expected_sha256();

        let output = HashedOutput {
            is_verified: true,

            actual_sha256: expected_sha256.to_owned(),
            expected_sha256: expected_sha256.to_owned(),
        };

        Ok(Some(output))
    }

    async fn init(
        &self,
        _prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Self::State> {
        let sha256_digest = Sha256::new();

        let state = sha256_digest;

        Ok(state)
    }

    async fn feed(&self, state: &mut Self::State, chunk: Bytes) -> anyhow::Result<()> {
        let sha256_digest = state;

        sha256_digest.update(chunk);

        Ok(())
    }

    async fn flush(&self, state: Self::State) -> anyhow::Result<Self::Staging> {
        let sha256_digest = state;

        let actual_sha256 = sha256_digest.finalize();
        let actual_sha256 = HexDisplay(&actual_sha256);
        let actual_sha256 = format!("{actual_sha256:x}");

        let staging = actual_sha256;

        Ok(staging)
    }

    async fn on_final_run(
        self,
        staging: Self::Staging,
        prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Self::Output> {
        let actual_sha256 = staging;

        let download = prepared_package.download();

        let expected_sha256 = download.expected_sha256();

        let is_verified = actual_sha256 == expected_sha256;

        if !is_verified {
            let err = anyhow!("Hasher failed due to SHA-256 mismatch");

            return Err(err);
        }

        let output = HashedOutput {
            is_verified,

            actual_sha256,
            expected_sha256: expected_sha256.to_owned(),
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
