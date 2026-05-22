pub(crate) mod handler;
pub(crate) mod pull_operator;
pub(crate) mod push_operator;

use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use anyhow::Result;
use frunk::{
    hlist::{HCons, HNil},
    traits::IntoReverse,
};
use futures::{
    future::TryFutureExt as _,
    sink::{self, SinkExt as _},
    stream::{self, StreamExt as _, TryStreamExt as _},
};
use tokio::task;
use tokio_util::{sync::PollSender, task::AbortOnDropHandle};

use crate::context::Context;

pub(crate) struct Pipeline<Item, St, Si, Handles> {
    stream: St,
    sink: Si,
    handles: Handles,

    context: Arc<Context>,

    _marker: PhantomData<Item>,
}

impl<Item, St> Pipeline<Item, St, sink::SinkErrInto<sink::Drain<Item>, Item, anyhow::Error>, HNil> {
    pub(crate) fn new(stream: St, context: Arc<Context>) -> Self {
        let sink = sink::drain();
        let sink = sink.sink_err_into();

        Self {
            stream,
            sink,
            handles: HNil,

            context,

            _marker: PhantomData,
        }
    }
}

impl<
    Item: Clone + Debug + Send + Sync + 'static,
    St: stream::TryStream<Ok = Item, Error = impl Into<anyhow::Error>> + Send + 'static,
    Si: sink::Sink<Item, Error = anyhow::Error> + Send + 'static,
    Handles: IntoReverse<Output: Collect>,
> Pipeline<Item, St, Si, Handles>
{
    #[expect(clippy::type_complexity)]
    pub(crate) fn fanout<Op: Operator<Item, _Marker>, _Marker>(
        self,
        operator: Op,
    ) -> Pipeline<
        Item,
        St,
        sink::Fanout<Si, sink::SinkErrInto<PollSender<Item>, Item, anyhow::Error>>,
        HCons<AbortOnDropHandle<Result<Op::Output>>, Handles>,
    > {
        let (sink, handle) = operator.spawn_blocking(&self.context);

        let sink = sink.sink_err_into();
        let sink = self.sink.fanout(sink);

        Pipeline {
            stream: self.stream,
            sink,
            handles: HCons {
                head: handle,
                tail: self.handles,
            },

            context: self.context,

            _marker: PhantomData,
        }
    }

    pub(crate) async fn run_parallel(self) -> Result<<Handles::Output as Collect>::Outputs> {
        let handle = task::spawn(async move {
            let stream = self.stream.err_into();

            let forward = stream.forward(self.sink);

            forward.await?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);
        let handle = handle.err_into();

        let handles = self.handles.into_reverse();
        let handles = handles.collect();

        let (result, outputs) = futures::try_join!(handle, handles)?;

        result?;

        Ok(outputs)
    }
}

pub(crate) trait Collect {
    type Outputs;

    async fn collect(self) -> Result<Self::Outputs>;
}

impl Collect for HNil {
    type Outputs = Self;

    async fn collect(self) -> Result<Self::Outputs> {
        let outputs = Self;

        Ok(outputs)
    }
}

impl<Item, Handles: Collect> Collect for HCons<AbortOnDropHandle<Result<Item>>, Handles> {
    type Outputs = HCons<Item, Handles::Outputs>;

    async fn collect(self) -> Result<Self::Outputs> {
        let head = self.head.err_into();

        let tail = self.tail.collect();

        let (output, outputs) = futures::try_join!(head, tail)?;

        let outputs = HCons {
            head: output?,
            tail: outputs,
        };

        Ok(outputs)
    }
}

pub(crate) trait Operator<Item, _Marker> {
    type Output;

    fn spawn_blocking(
        self,
        context: &Context,
    ) -> (PollSender<Item>, AbortOnDropHandle<Result<Self::Output>>);
}
