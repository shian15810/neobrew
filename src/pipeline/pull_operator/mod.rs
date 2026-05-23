mod temp_pourer;

use std::io::{self, BufRead};

use anyhow::Result;
use bytes::Buf;
use futures::stream::StreamExt as _;
use tokio::{sync::mpsc, task};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::{
    io::{StreamReader, SyncIoBridge},
    sync::PollSender,
    task::AbortOnDropHandle,
};

pub(crate) use self::temp_pourer::{TempPourer, TempPourerInput};
use super::Operator;
use crate::context::Context;

pub(crate) trait PullOperator {
    type Output;

    #[expect(clippy::wrong_self_convention)]
    fn from_reader(self, reader: impl BufRead) -> Result<Self::Output>;
}

impl<
    Item: Buf + Send + 'static,
    Output: Send + 'static,
    PullOp: PullOperator<Output = Output> + Send + 'static,
> Operator<Item, PullMarker> for PullOp
{
    type Output = PullOp::Output;

    fn spawn_blocking(
        self,
        context: &Context,
    ) -> (PollSender<Item>, AbortOnDropHandle<Result<Self::Output>>) {
        let (tx, rx) = mpsc::channel(*context.channel_capacity);

        let sink = PollSender::new(tx);

        let stream = ReceiverStream::new(rx);
        let stream = stream.map(io::Result::Ok);

        let reader = StreamReader::new(stream);

        let sync_reader = SyncIoBridge::new(reader);

        let handle = task::spawn_blocking(move || {
            let output = self.from_reader(sync_reader)?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}

pub(crate) struct PullMarker;
