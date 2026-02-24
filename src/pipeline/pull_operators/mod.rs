use std::io::{self, BufRead};

use anyhow::Result;
use bytes::Buf;
use futures::stream::StreamExt;
use tokio::{sync::mpsc, task};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::{
    io::{StreamReader, SyncIoBridge},
    sync::PollSender,
    task::AbortOnDropHandle,
};

pub use self::pourer::Pourer;
use super::Operator;
use crate::context::Context;

mod pourer;

pub struct PullMarker;

pub trait PullOperator {
    type Output;

    fn from_reader(self, reader: impl BufRead) -> Result<Self::Output>;
}

impl<
    Item: Buf + Send + 'static,
    Output: Send + 'static,
    PullOp: PullOperator<Output = Output> + Send + 'static,
> Operator<Item, PullMarker> for PullOp
{
    type Output = PullOp::Output;

    fn spawn(
        self,
        context: &Context,
    ) -> (PollSender<Item>, AbortOnDropHandle<Result<Self::Output>>) {
        let (tx, rx) = mpsc::channel(*context.channel_capacity);

        let sink = PollSender::new(tx);

        let stream = ReceiverStream::new(rx).map(Ok::<_, io::Error>);

        let reader = StreamReader::new(stream);

        let sync_reader = SyncIoBridge::new(reader);

        let handle = task::spawn_blocking(move || {
            let output = self.from_reader(sync_reader)?;

            Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}
