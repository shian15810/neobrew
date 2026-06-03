use tokio::sync::watch;

pub(in super::super) struct PipelineChannels {
    pub(in super::super) is_verified_tx: watch::Sender<Option<bool>>,
    pub(in super::super) is_verified_rx: watch::Receiver<Option<bool>>,
}

impl PipelineChannels {
    pub(super) fn new() -> Self {
        let (is_verified_tx, is_verified_rx) = watch::channel(None);

        Self {
            is_verified_tx,
            is_verified_rx,
        }
    }
}
