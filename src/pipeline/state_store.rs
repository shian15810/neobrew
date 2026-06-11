use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::Duration,
};

use indicatif::ProgressBar;
use tokio::sync::watch;

use crate::{
    context::Context,
    ext::std::sync::OnceLockExt as _,
    package::prepared::PreparedPackage,
    util::ArchiveFormat,
};

#[derive(Clone)]
pub(crate) struct Session {
    pub(super) channel: Channel,

    pub(super) prepared_package: Arc<PreparedPackage>,

    pub(super) pb: ProgressBar,

    pub(super) context: Arc<Context>,
}

impl Session {
    pub(super) fn new(
        prepared_package: PreparedPackage,
        pb: ProgressBar,
        context: Arc<Context>,
    ) -> Self {
        Self {
            channel: Channel::new(),

            prepared_package: Arc::new(prepared_package),

            pb,

            context,
        }
    }
}

#[derive(Clone)]
pub(super) struct Channel {
    pub(super) state_store_tx: watch::Sender<StateStore>,
    pub(super) state_store_rx: watch::Receiver<StateStore>,
}

impl Channel {
    pub(super) fn new() -> Self {
        let state_store = StateStore::default();

        let (state_store_tx, state_store_rx) = watch::channel(state_store);

        Self {
            state_store_tx,
            state_store_rx,
        }
    }
}

#[derive(Default)]
pub(crate) struct StateStore {
    pub(super) stage: Stage,
    pub(super) payloads: Arc<Payloads>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Stage {
    #[default]
    Streaming,
    Progressed,
    Hashed,
    Written,
    Poured,
    Relocated,
    Linked,
    Artifacted,
}

#[derive(Default)]
pub(super) struct Payloads {
    streaming: (),
    pub(super) progressed: OnceLock<ProgressedOutput>,
    pub(super) hashed: OnceLock<HashedOutput>,
    pub(super) written: OnceLock<WrittenOutput>,
    pub(super) poured: OnceLock<PouredOutput>,
    pub(super) relocated: OnceLock<RelocatedOutput>,
    pub(super) linked: OnceLock<LinkedOutput>,
    pub(super) artifacted: OnceLock<ArtifactedOutput>,
}

pub(super) trait Publish<Output> {
    fn publish(&self, output: &Output) -> anyhow::Result<()>;
}

pub(super) trait Subscribe<Payload> {
    fn subscribe(&self) -> anyhow::Result<&Payload>;
}

#[derive(Clone)]
pub(crate) struct ProgressedOutput {
    pub(super) position: u64,
    pub(super) length: Option<u64>,
    pub(super) per_sec: f64,
    pub(super) elapsed: Duration,
}

impl Publish<ProgressedOutput> for Payloads {
    fn publish(&self, output: &ProgressedOutput) -> anyhow::Result<()> {
        self.progressed.try_set(output.clone())?;

        Ok(())
    }
}

impl Subscribe<ProgressedOutput> for Payloads {
    fn subscribe(&self) -> anyhow::Result<&ProgressedOutput> {
        let payload = self.progressed.try_get()?;

        Ok(payload)
    }
}

#[derive(Clone)]
pub(crate) struct HashedOutput {
    pub(super) is_verified: bool,

    pub(super) actual_sha256: String,
    pub(super) expected_sha256: String,
}

impl Publish<HashedOutput> for Payloads {
    fn publish(&self, output: &HashedOutput) -> anyhow::Result<()> {
        self.hashed.try_set(output.clone())?;

        Ok(())
    }
}

impl Subscribe<HashedOutput> for Payloads {
    fn subscribe(&self) -> anyhow::Result<&HashedOutput> {
        let payload = self.hashed.try_get()?;

        Ok(payload)
    }
}

#[derive(Clone)]
pub(crate) struct WrittenOutput {
    pub(super) dest_file_path: PathBuf,
    pub(super) dest_link_path: PathBuf,
}

impl Publish<WrittenOutput> for Payloads {
    fn publish(&self, output: &WrittenOutput) -> anyhow::Result<()> {
        self.written.try_set(output.clone())?;

        Ok(())
    }
}

impl Subscribe<WrittenOutput> for Payloads {
    fn subscribe(&self) -> anyhow::Result<&WrittenOutput> {
        let payload = self.written.try_get()?;

        Ok(payload)
    }
}

#[derive(Clone)]
pub(crate) struct PouredOutput {
    pub(super) dest_dir_path: PathBuf,
    pub(super) archive_format: ArchiveFormat,
}

impl Publish<PouredOutput> for Payloads {
    fn publish(&self, output: &PouredOutput) -> anyhow::Result<()> {
        self.poured.try_set(output.clone())?;

        Ok(())
    }
}

impl Subscribe<PouredOutput> for Payloads {
    fn subscribe(&self) -> anyhow::Result<&PouredOutput> {
        let payload = self.poured.try_get()?;

        Ok(payload)
    }
}

#[derive(Clone)]
pub(crate) struct RelocatedOutput {
    pub(super) keg_dir_path: PathBuf,
}

impl Publish<RelocatedOutput> for Payloads {
    fn publish(&self, output: &RelocatedOutput) -> anyhow::Result<()> {
        self.relocated.try_set(output.clone())?;

        Ok(())
    }
}

impl Subscribe<RelocatedOutput> for Payloads {
    fn subscribe(&self) -> anyhow::Result<&RelocatedOutput> {
        let payload = self.relocated.try_get()?;

        Ok(payload)
    }
}

#[derive(Clone)]
pub(crate) struct LinkedOutput {
    pub(super) opt_prefix_link_path: PathBuf,
    pub(super) linked_keg_prefix_link_path: Option<PathBuf>,
}

impl Publish<LinkedOutput> for Payloads {
    fn publish(&self, output: &LinkedOutput) -> anyhow::Result<()> {
        self.linked.try_set(output.clone())?;

        Ok(())
    }
}

impl Subscribe<LinkedOutput> for Payloads {
    fn subscribe(&self) -> anyhow::Result<&LinkedOutput> {
        let payload = self.linked.try_get()?;

        Ok(payload)
    }
}

#[derive(Clone)]
pub(crate) struct ArtifactedOutput {
    pub(super) staged_dir_path: PathBuf,
}

impl Publish<ArtifactedOutput> for Payloads {
    fn publish(&self, output: &ArtifactedOutput) -> anyhow::Result<()> {
        self.artifacted.try_set(output.clone())?;

        Ok(())
    }
}

impl Subscribe<ArtifactedOutput> for Payloads {
    fn subscribe(&self) -> anyhow::Result<&ArtifactedOutput> {
        let payload = self.artifacted.try_get()?;

        Ok(payload)
    }
}
