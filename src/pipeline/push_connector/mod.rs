pub(crate) mod hasher;
pub(crate) mod progressor;
pub(crate) mod writer;

use async_trait::async_trait;
use bytes::Bytes;
use futures::sink::{self, SinkExt as _};
use tokio::{sync::mpsc, task};
use tokio_util::{sync::PollSender, task::AbortOnDropHandle};

use super::{
    Connector,
    state_committer::StateCommitter,
    state_store::{Payloads, Publish, Session, Stage},
};
use crate::package::prepared::{PreparedPackage, download::Download};

pub(crate) struct _PushConnectorMarker;

#[async_trait]
pub(crate) trait PushConnector: Sized {
    type State;
    type Staging;
    type Output;

    fn should_run(&self, _prepared_package: &PreparedPackage<Download>) -> bool {
        true
    }

    async fn on_skip_run(
        self,
        _prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Option<Self::Output>> {
        Ok(None)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        None
    }

    async fn init(
        &self,
        prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Self::State>;

    async fn feed(&self, state: &mut Self::State, chunk: Bytes) -> anyhow::Result<()>;

    async fn flush(&self, state: Self::State) -> anyhow::Result<Self::Staging>;

    fn wait_stage(&self) -> Option<Stage> {
        None
    }

    async fn on_final_run(
        self,
        staging: Self::Staging,
        prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Self::Output>;

    fn passed_prefix(&self) -> Option<&'static str> {
        None
    }

    fn failed_prefix(&self) -> Option<&'static str> {
        None
    }

    fn passed_stage(&self, should_run: bool) -> Option<Stage>;
}

impl<
    Output: Send + 'static,
    PushConn: PushConnector<State: Send, Staging: Send, Output = Output> + Send + 'static,
> Connector<_PushConnectorMarker> for PushConn
where
    Payloads: Publish<PushConn::Output>,
{
    type Sink = sink::SinkErrInto<PollSender<Bytes>, Bytes, anyhow::Error>;
    type Output = PushConn::Output;

    fn launch(
        self,
        mut session: Session,
    ) -> (
        Self::Sink,
        AbortOnDropHandle<anyhow::Result<Option<Self::Output>>>,
    ) {
        let context = &session.context;

        let (tx, mut rx) = mpsc::channel(context.channel_capacity);

        let sink = PollSender::new(tx);
        let sink = sink.sink_err_into();

        let handle = task::spawn(async move {
            let channel = &mut session.channel;

            let prepared_package = &session.prepared_package;

            let pb = &session.pb;

            let _context = &session.context;

            let should_run = self.should_run(prepared_package);

            let state_committer = StateCommitter {
                passed_prefix: self.passed_prefix(),
                failed_prefix: self.failed_prefix(),

                passed_stage: self.passed_stage(should_run),
            };

            if !should_run {
                while rx.recv().await.is_some() {}

                let output_res = self.on_skip_run(prepared_package).await;

                let output = state_committer.finalize(output_res, &session)?;

                return Ok(output);
            }

            if let Some(running_prefix) = self.running_prefix() {
                pb.set_prefix(running_prefix);
            }

            let mut state = self.init(prepared_package).await?;

            while let Some(item) = rx.recv().await {
                self.feed(&mut state, item).await?;
            }

            let staging = self.flush(state).await?;

            if let Some(wait_stage) = self.wait_stage() {
                channel
                    .state_store_rx
                    .wait_for(|state_store| state_store.stage >= wait_stage)
                    .await?;
            }

            let output_res = self.on_final_run(staging, prepared_package).await;
            let output_res = output_res.map(Some);

            let output = state_committer.finalize(output_res, &session)?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}
