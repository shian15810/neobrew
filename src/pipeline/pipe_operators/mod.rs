use std::io::{self, BufRead};

use anyhow::Result;
use bytes::Buf;
use futures::stream::StreamExt;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::{
    io::{StreamReader, SyncIoBridge},
    sync::PollSender,
};

pub use self::pourer::Pourer;
use super::Operator;
use crate::context::Context;

mod pourer;

pub struct PipeMarker;

pub trait PipeOperator {
    type Output;

    fn from_reader(self, reader: impl BufRead) -> Result<Self::Output>;
}

impl<
    Item: Buf + Send + 'static,
    Output: Send + 'static,
    PipeOp: PipeOperator<Output = Output> + Send + 'static,
> Operator<Item, PipeMarker> for PipeOp
{
    type Output = PipeOp::Output;

    fn spawn(
        self,
        set: &mut JoinSet<Result<()>>,
        context: &Context,
    ) -> (PollSender<Item>, oneshot::Receiver<Self::Output>) {
        let channel_capacity = context.channel_capacity();

        let (input_tx, input_rx) = mpsc::channel(channel_capacity);

        let (output_tx, output_rx) = oneshot::channel();

        let sink = PollSender::new(input_tx);

        let stream = ReceiverStream::new(input_rx).map(Ok::<_, io::Error>);

        let reader = StreamReader::new(stream);

        let sync_reader = SyncIoBridge::new(reader);

        set.spawn_blocking(move || {
            let output = self.from_reader(sync_reader)?;

            let _ = output_tx.send(output);

            Ok(())
        });

        (sink, output_rx)
    }
}
