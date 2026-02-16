use std::{pin::Pin, sync::Arc, task};

use anyhow::Result;
use futures::sink::{self, SinkExt};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_util::sync::{PollSendError, PollSender};

pub use self::{hasher::Hasher, writer::Writer};
use crate::context::Context;

mod hasher;
mod writer;

pub trait Operator<Item: Send + 'static, Output: Send + 'static>: Send + 'static {
    fn spawn(
        self,
        set: &mut JoinSet<Result<()>>,
        context: Arc<Context>,
    ) -> (BlockingSink<Item>, oneshot::Receiver<Output>)
    where
        Self: Sized,
    {
        let (output_tx, output_rx) = oneshot::channel();

        let sink = BlockingSink::new(self, output_tx, set, context);

        (sink, output_rx)
    }

    fn feed(&mut self, item: Item) -> Result<()>;

    fn flush(self) -> Result<Output>;
}

pub struct BlockingSink<Item> {
    inner: PollSender<Item>,
}

impl<Item: Send + 'static> BlockingSink<Item> {
    fn new<Output: Send + 'static>(
        mut operator: impl Operator<Item, Output>,
        output_tx: oneshot::Sender<Output>,
        set: &mut JoinSet<Result<()>>,
        _context: Arc<Context>,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel(32);

        set.spawn_blocking(move || {
            while let Some(item) = rx.blocking_recv() {
                operator.feed(item)?;
            }

            let output = operator.flush()?;

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
