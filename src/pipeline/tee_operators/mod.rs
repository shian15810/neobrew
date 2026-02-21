use std::{pin::Pin, task};

use anyhow::Result;
use futures::sink::{self, SinkExt};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_util::sync::{PollSendError, PollSender};

pub use self::{hasher::Hasher, writer::Writer};
use super::Operator;
use crate::context::Context;

mod hasher;
mod writer;

pub trait TeeOperator<Item, Output> {
    fn feed(&mut self, chunk: Item) -> Result<()>;

    fn flush(self) -> Result<Output>;
}

impl<
    Item: Send + 'static,
    Output: Send + 'static,
    TeeOp: TeeOperator<Item, Output> + Send + 'static,
> Operator<BlockingSink<Item>, Output> for TeeOp
{
    fn spawn(
        self,
        set: &mut JoinSet<Result<()>>,
        context: &Context,
    ) -> (BlockingSink<Item>, oneshot::Receiver<Output>) {
        let (output_tx, output_rx) = oneshot::channel();

        let sink = BlockingSink::new(self, output_tx, set, context);

        (sink, output_rx)
    }
}

pub struct BlockingSink<Item> {
    inner: PollSender<Item>,
}

impl<Item: Send + 'static> BlockingSink<Item> {
    fn new<Output: Send + 'static>(
        mut tee_operator: impl TeeOperator<Item, Output> + Send + 'static,
        output_tx: oneshot::Sender<Output>,
        set: &mut JoinSet<Result<()>>,
        context: &Context,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel(context.channel_capacity());

        set.spawn_blocking(move || {
            while let Some(item) = rx.blocking_recv() {
                tee_operator.feed(item)?;
            }

            let output = tee_operator.flush()?;

            let _ = output_tx.send(output);

            Ok(())
        });

        Self {
            inner: PollSender::new(tx),
        }
    }
}

impl<Item: Send> sink::Sink<Item> for BlockingSink<Item> {
    type Error = PollSendError<Item>;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context,
    ) -> task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready_unpin(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        self.inner.start_send_unpin(item)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context,
    ) -> task::Poll<Result<(), Self::Error>> {
        self.inner.poll_flush_unpin(cx)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context,
    ) -> task::Poll<Result<(), Self::Error>> {
        self.inner.poll_close_unpin(cx)
    }
}
