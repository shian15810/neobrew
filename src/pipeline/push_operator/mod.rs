mod hasher;
mod temp_writer;

use anyhow::Result;
use tokio::{sync::mpsc, task};
use tokio_util::{sync::PollSender, task::AbortOnDropHandle};

pub(crate) use self::{hasher::Hasher, temp_writer::TempWriter};
use super::Operator;
use crate::context::Context;

#[trait_variant::make(Send)]
pub(crate) trait PushOperator {
    type Item;
    type Output;

    async fn feed(&mut self, chunk: Self::Item) -> Result<()>;

    async fn flush(self) -> Result<Self::Output>;
}

impl<
    Item: Send + 'static,
    Output: Send + 'static,
    PushOp: PushOperator<Item = Item, Output = Output> + Send + 'static,
> Operator<Item, PushMarker> for PushOp
{
    type Output = PushOp::Output;

    fn launch(
        mut self,
        context: &Context,
    ) -> (PollSender<Item>, AbortOnDropHandle<Result<Self::Output>>) {
        let (tx, mut rx) = mpsc::channel(*context.channel_capacity);

        let sink = PollSender::new(tx);

        let handle = task::spawn(async move {
            while let Some(item) = rx.recv().await {
                self.feed(item).await?;
            }

            let output = self.flush().await?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}

pub(crate) struct PushMarker;
