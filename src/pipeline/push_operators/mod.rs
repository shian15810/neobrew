use anyhow::Result;
use tokio::{sync::mpsc, task};
use tokio_util::{sync::PollSender, task::AbortOnDropHandle};

pub use self::{hasher::Hasher, writer::Writer};
use super::Operator;
use crate::context::Context;

mod hasher;
mod writer;

pub struct PushMarker;

pub trait PushOperator {
    type Item;
    type Output;

    fn feed(&mut self, chunk: Self::Item) -> Result<()>;

    fn flush(self) -> Result<Self::Output>;
}

impl<
    Item: Send + 'static,
    Output: Send + 'static,
    PushOp: PushOperator<Item = Item, Output = Output> + Send + 'static,
> Operator<Item, PushMarker> for PushOp
{
    type Output = PushOp::Output;

    fn spawn(
        mut self,
        context: &Context,
    ) -> (PollSender<Item>, AbortOnDropHandle<Result<Self::Output>>) {
        let (tx, mut rx) = mpsc::channel(context.channel_capacity());

        let sink = PollSender::new(tx);

        let handle = task::spawn_blocking(move || {
            while let Some(item) = rx.blocking_recv() {
                self.feed(item)?;
            }

            let output = self.flush()?;

            Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}
