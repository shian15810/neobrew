mod temp_pourer;

use std::sync::Arc;

use bytes::Buf;
use futures::stream::StreamExt as _;
use tokio::{
    io::{self, AsyncRead},
    sync::mpsc,
    task,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::{io::StreamReader, sync::PollSender, task::AbortOnDropHandle};

pub(crate) use self::temp_pourer::TempPourer;
use super::{Operator, channels::PipelineChannels as Channels};
use crate::context::Context;

#[trait_variant::make(Send)]
pub(crate) trait PullOperator {
    type Output;

    #[expect(clippy::wrong_self_convention)]
    async fn from_reader(&self, reader: impl AsyncRead + Unpin + Send) -> anyhow::Result<()>;

    async fn after_drain(
        self,
        channels: Arc<Channels>,
        context: Arc<Context>,
    ) -> anyhow::Result<Self::Output>;

    fn cleanup(self) -> anyhow::Result<()>;

    async fn persist(self) -> anyhow::Result<()>;
}

impl<
    Item: Buf + Send + 'static,
    Output: Send + 'static,
    PullOp: PullOperator<Output = Output> + Send + 'static,
> Operator<Item, PullMarker> for PullOp
{
    type Output = PullOp::Output;

    fn launch(
        self,
        channels: Arc<Channels>,
        context: Arc<Context>,
    ) -> (
        PollSender<Item>,
        AbortOnDropHandle<anyhow::Result<Self::Output>>,
    ) {
        let (tx, rx) = mpsc::channel(context.channel_capacity);

        let sink = PollSender::new(tx);

        let stream = ReceiverStream::new(rx);
        let stream = stream.map(io::Result::Ok);

        let mut reader = StreamReader::new(stream);

        let handle = task::spawn(async move {
            self.from_reader(&mut reader).await?;

            io::copy(&mut reader, &mut io::sink()).await?;

            let output = self.after_drain(channels, context).await?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}

pub(crate) struct PullMarker;
