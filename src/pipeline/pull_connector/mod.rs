mod pourer;

use std::fmt::Debug;

use anyhow::anyhow;
use async_trait::async_trait;
use bytes::Buf;
use futures::{
    future::Either::{self, Left, Right},
    sink::{self, SinkExt as _},
    stream::StreamExt as _,
};
use tokio::{
    io::{self, AsyncRead},
    sync::{mpsc, watch},
    task,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::{io::StreamReader, sync::PollSender, task::AbortOnDropHandle};

pub(crate) use self::pourer::Pourer;
use super::{
    Connector,
    state_committer::StateCommitter,
    state_store::{Payloads, Publish, Session, Stage, StateStore},
};

pub(crate) struct _PullConnectorMarker;

#[async_trait]
pub(crate) trait PullConnector: Sized {
    type Staging;
    type Output;

    fn should_run(&self) -> bool {
        true
    }

    async fn on_skip_run(
        self,
        _state_store_rx: &mut watch::Receiver<StateStore>,
    ) -> anyhow::Result<Self::Output> {
        let err = anyhow!("Implement `on_skip_run` when `should_run` returns `false`");

        Err(err)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        None
    }

    #[expect(clippy::wrong_self_convention)]
    async fn from_reader(
        &self,
        reader: &mut (impl AsyncRead + Unpin + Send),
    ) -> anyhow::Result<Self::Staging>;

    async fn on_final_run(
        self,
        staging: Self::Staging,
        state_store_rx: &mut watch::Receiver<StateStore>,
    ) -> anyhow::Result<Self::Output>;

    async fn persist(self) -> anyhow::Result<()> {
        Ok(())
    }

    fn cleanup(self) -> anyhow::Result<()> {
        Ok(())
    }

    fn passed_prefix(&self, _should_run: bool) -> Option<&'static str> {
        None
    }

    fn failed_prefix(&self, _should_run: bool) -> Option<&'static str> {
        None
    }

    fn passed_stage(&self, should_run: bool) -> Option<Stage>;
}

impl<
    Item: Buf + Debug + Send + Sync + 'static,
    Output: Send + 'static,
    PullConn: PullConnector<Staging: Send, Output = Output> + Send + 'static,
> Connector<Item, _PullConnectorMarker> for PullConn
where
    Payloads: Publish<PullConn::Output>,
{
    type Sink = Either<
        sink::SinkErrInto<sink::Drain<Item>, Item, anyhow::Error>,
        sink::SinkErrInto<PollSender<Item>, Item, anyhow::Error>,
    >;
    type Output = PullConn::Output;

    fn launch(
        self,
        mut session: Session,
    ) -> (Self::Sink, AbortOnDropHandle<anyhow::Result<Self::Output>>) {
        let context = &session.context;

        let should_run = self.should_run();

        if !should_run {
            let sink = sink::drain();
            let sink = sink.sink_err_into();

            let handle = task::spawn(async move {
                let channel = &mut session.channel;

                let state_committer = StateCommitter {
                    passed_stage: self.passed_stage(should_run),

                    passed_prefix: self.passed_prefix(should_run),
                    failed_prefix: self.failed_prefix(should_run),
                };

                let output = self.on_skip_run(&mut channel.state_store_rx).await;
                let output = state_committer.finalize(output, &session)?;

                anyhow::Ok(output)
            });
            let handle = AbortOnDropHandle::new(handle);

            return (Left(sink), handle);
        }

        let (tx, rx) = mpsc::channel(context.channel_capacity);

        let sink = PollSender::new(tx);
        let sink = sink.sink_err_into();

        let stream = ReceiverStream::new(rx);
        let stream = stream.map(io::Result::Ok);

        let mut reader = StreamReader::new(stream);

        let handle = task::spawn(async move {
            let channel = &mut session.channel;

            let pb = &session.pb;

            if let Some(running_prefix) = self.running_prefix() {
                pb.set_prefix(running_prefix);
            }

            let staging = self.from_reader(&mut reader).await?;

            let mut stream = reader.into_inner();

            while stream.next().await.is_some() {}

            let state_committer = StateCommitter {
                passed_stage: self.passed_stage(should_run),

                passed_prefix: self.passed_prefix(should_run),
                failed_prefix: self.failed_prefix(should_run),
            };

            let output = self
                .on_final_run(staging, &mut channel.state_store_rx)
                .await;
            let output = state_committer.finalize(output, &session)?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (Right(sink), handle)
    }
}
