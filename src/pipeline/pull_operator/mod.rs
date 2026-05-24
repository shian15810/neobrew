mod temp_pourer;

use anyhow::Result;
use bytes::Buf;
use futures::stream::StreamExt as _;
use tokio::{
    io::{self, AsyncRead},
    sync::mpsc,
    task,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::{io::StreamReader, sync::PollSender, task::AbortOnDropHandle};

pub(crate) use self::temp_pourer::{TempPourer, TempPourerInput};
use super::Operator;
use crate::context::Context;

#[trait_variant::make(Send)]
pub(crate) trait PullOperator {
    type Output;

    #[expect(clippy::wrong_self_convention)]
    async fn from_reader(self, reader: impl AsyncRead + Unpin + Send) -> Result<Self::Output>;
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
        context: &Context,
    ) -> (PollSender<Item>, AbortOnDropHandle<Result<Self::Output>>) {
        let (tx, rx) = mpsc::channel(*context.channel_capacity);

        let sink = PollSender::new(tx);

        let stream = ReceiverStream::new(rx);
        let stream = stream.map(io::Result::Ok);

        let reader = StreamReader::new(stream);

        let handle = task::spawn(async move {
            let output = self.from_reader(reader).await?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}

pub(crate) struct PullMarker;
