use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::Duration,
};

use tokio::sync::watch;

use crate::{ext::std::sync::OnceLockExt as _, util::ArchiveFormat};

#[derive(Clone)]
pub(crate) struct Channel {
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
    pub(super) outputs: Arc<Outputs>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Stage {
    #[default]
    Streaming,
    Progressed,
    Hashed,
    Written,
    Poured,
}

#[derive(Default)]
pub(super) struct Outputs {
    streaming: (),
    pub(super) progressed: OnceLock<ProgressedOutput>,
    pub(super) hashed: OnceLock<HashedOutput>,
    pub(super) written: OnceLock<WrittenOutput>,
    pub(super) poured: OnceLock<PouredOutput>,
}

pub(super) trait Publish {
    fn publish(&self, outputs: &Outputs) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub(crate) struct ProgressedOutput {
    pub(super) position: u64,
    pub(super) length: Option<u64>,
    pub(super) per_sec: f64,
    pub(super) elapsed: Duration,
}

impl Publish for ProgressedOutput {
    fn publish(&self, outputs: &Outputs) -> anyhow::Result<()> {
        outputs.progressed.try_set(self.clone())?;

        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct HashedOutput {
    pub(super) is_verified: bool,

    pub(super) actual_sha256: String,
    pub(super) expected_sha256: String,
}

impl Publish for HashedOutput {
    fn publish(&self, outputs: &Outputs) -> anyhow::Result<()> {
        outputs.hashed.try_set(self.clone())?;

        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct WrittenOutput {
    pub(super) dest_file_path: PathBuf,
    pub(super) dest_link_path: PathBuf,
}

impl Publish for WrittenOutput {
    fn publish(&self, outputs: &Outputs) -> anyhow::Result<()> {
        outputs.written.try_set(self.clone())?;

        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct PouredOutput {
    pub(super) dest_dir_path: PathBuf,
    pub(super) archive_format: ArchiveFormat,
}

impl Publish for PouredOutput {
    fn publish(&self, outputs: &Outputs) -> anyhow::Result<()> {
        outputs.poured.try_set(self.clone())?;

        Ok(())
    }
}
