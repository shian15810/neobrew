mod pourer;

use async_trait::async_trait;
use bytes::Bytes;
use futures::{
    sink::{self, SinkExt as _},
    stream::StreamExt as _,
};
use tokio::{
    io::{self, AsyncRead},
    sync::mpsc,
    task,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::{io::StreamReader, sync::PollSender, task::AbortOnDropHandle};

pub(crate) use self::pourer::Pourer;
use super::{
    Connector,
    state_committer::StateCommitter,
    state_store::{Payloads, Publish, Session, Stage},
};

pub(crate) struct _PullConnectorMarker;

#[async_trait]
pub(crate) trait PullConnector: Sized {
    type Staging;
    type Output;

    fn should_run(&self) -> bool {
        true
    }

    fn on_skip_run(self) -> anyhow::Result<Option<Self::Output>> {
        Ok(None)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        None
    }

    #[expect(clippy::wrong_self_convention)]
    async fn from_reader(
        &self,
        reader: &mut (impl AsyncRead + Unpin + Send),
    ) -> anyhow::Result<Self::Staging>;

    fn wait_stage(&self) -> Option<Stage> {
        None
    }

    async fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output>;

    async fn persist(self) -> anyhow::Result<()> {
        Ok(())
    }

    fn cleanup(self) -> anyhow::Result<()> {
        Ok(())
    }

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
    PullConn: PullConnector<Staging: Send, Output = Output> + Send + 'static,
> Connector<_PullConnectorMarker> for PullConn
where
    Payloads: Publish<PullConn::Output>,
{
    type Sink = sink::SinkErrInto<PollSender<Bytes>, Bytes, anyhow::Error>;
    type Output = PullConn::Output;

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

            let pb = &session.pb;

            let should_run = self.should_run();

            let state_committer = StateCommitter {
                passed_prefix: self.passed_prefix(),
                failed_prefix: self.failed_prefix(),

                passed_stage: self.passed_stage(should_run),
            };

            if !should_run {
                while rx.recv().await.is_some() {}

                let output = self.on_skip_run();
                let output = state_committer.finalize(output, &session)?;

                return Ok(output);
            }

            if let Some(running_prefix) = self.running_prefix() {
                pb.set_prefix(running_prefix);
            }

            let stream = ReceiverStream::new(rx);
            let stream = stream.map(io::Result::Ok);

            let mut reader = StreamReader::new(stream);

            let staging = self.from_reader(&mut reader).await?;

            let mut stream = reader.into_inner();

            while stream.next().await.is_some() {}

            if let Some(wait_stage) = self.wait_stage() {
                channel
                    .state_store_rx
                    .wait_for(|state_store| state_store.stage >= wait_stage)
                    .await?;
            }

            let output = self.on_final_run(staging).await;
            let output = output.map(Some);
            let output = state_committer.finalize(output, &session)?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}
