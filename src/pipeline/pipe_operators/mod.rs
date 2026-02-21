use std::io::{self, BufRead};

use anyhow::Result;
use bytes::Buf;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::io::{StreamReader, SyncIoBridge};

pub use self::pourer::Pourer;
use super::Operator;
use crate::context::Context;

mod pourer;

pub trait PipeOperator<Output> {
    fn from_reader(self, reader: impl BufRead) -> Result<Output>;
}

impl<
    Item: Buf + Send + 'static,
    Output: Send + 'static,
    PipeOp: PipeOperator<Output> + Send + 'static,
> Operator<mpsc::Sender<io::Result<Item>>, Output> for PipeOp
{
    fn spawn(
        self,
        set: &mut JoinSet<Result<()>>,
        context: &Context,
    ) -> (mpsc::Sender<io::Result<Item>>, oneshot::Receiver<Output>) {
        let (input_tx, input_rx) = mpsc::channel(context.channel_capacity());

        let (output_tx, output_rx) = oneshot::channel();

        let stream = ReceiverStream::new(input_rx);

        let reader = StreamReader::new(stream);

        let sync_reader = SyncIoBridge::new(reader);

        set.spawn_blocking(move || {
            let output = self.from_reader(sync_reader)?;

            let _ = output_tx.send(output);

            Ok(())
        });

        (input_tx, output_rx)
    }
}
