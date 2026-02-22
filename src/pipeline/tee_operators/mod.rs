use anyhow::Result;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};
use tokio_util::sync::PollSender;

pub use self::{hasher::Hasher, writer::Writer};
use super::Operator;
use crate::context::Context;

mod hasher;
mod writer;

pub struct TeeMarker;

pub trait TeeOperator {
    type Item;
    type Output;

    fn feed(&mut self, chunk: Self::Item) -> Result<()>;

    fn flush(self) -> Result<Self::Output>;
}

impl<
    Item: Send + 'static,
    Output: Send + 'static,
    TeeOp: TeeOperator<Item = Item, Output = Output> + Send + 'static,
> Operator<Item, TeeMarker> for TeeOp
{
    type Output = TeeOp::Output;

    fn spawn(
        mut self,
        set: &mut JoinSet<Result<()>>,
        context: &Context,
    ) -> (PollSender<Item>, oneshot::Receiver<Self::Output>) {
        let channel_capacity = context.channel_capacity();

        let (input_tx, mut input_rx) = mpsc::channel(channel_capacity);

        let (output_tx, output_rx) = oneshot::channel();

        let sink = PollSender::new(input_tx);

        set.spawn_blocking(move || {
            while let Some(item) = input_rx.blocking_recv() {
                self.feed(item)?;
            }

            let output = self.flush()?;

            let _ = output_tx.send(output);

            Ok(())
        });

        (sink, output_rx)
    }
}
