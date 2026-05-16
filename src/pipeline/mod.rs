use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use anyhow::{Error, Result};
use frunk::{
    hlist::{HCons, HNil},
    traits::IntoReverse,
};
use futures::{
    future::TryFutureExt as _,
    sink::{self, SinkExt as _},
    stream::{self, StreamExt as _, TryStreamExt as _},
};
use tokio::task::{self, JoinHandle};
use tokio_util::{sync::PollSender, task::AbortOnDropHandle};

pub(crate) use self::{
    pull_operators::Pourer,
    push_operators::{Hasher, Writer},
};
use crate::context::Context;

mod pull_operators;
mod push_operators;

pub(crate) struct Pipeline<Item, St, Si, Handles> {
    stream: St,
    sink: Si,
    handles: Handles,

    context: Arc<Context>,

    _marker: PhantomData<Item>,
}

impl<Item, St> Pipeline<Item, St, sink::SinkErrInto<sink::Drain<Item>, Item, Error>, HNil> {
    pub(crate) fn new(stream: St, context: Arc<Context>) -> Self {
        Self {
            stream,
            sink: sink::drain().sink_err_into(),
            handles: HNil,

            context,

            _marker: PhantomData,
        }
    }
}

impl<
    Item: Clone + Debug + Send + Sync + 'static,
    St: stream::TryStream<Ok = Item, Error = impl Into<Error>> + Send + 'static,
    Si: sink::Sink<Item, Error = Error> + Send + 'static,
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
        sink::Fanout<Si, sink::SinkErrInto<PollSender<Item>, Item, Error>>,
        HCons<AbortOnDropHandle<Result<Op::Output>>, Handles>,
    > {
        let context = Arc::as_ref(&self.context);

        let (sink, handle) = operator.spawn_blocking(context);

        Pipeline {
            stream: self.stream,
            sink: self.sink.fanout(sink.sink_err_into()),
            handles: HCons {
                head: handle,
                tail: self.handles,
            },

            context: self.context,

            _marker: PhantomData,
        }
    }

    pub(crate) async fn run_parallel(self) -> Result<<Handles::Output as Collect>::Outputs> {
        let handle: JoinHandle<Result<()>> = task::spawn(async move {
            let forward = self.stream.err_into().forward(self.sink);

            forward.await?;

            Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        let (result, outputs) =
            futures::try_join!(handle.err_into(), self.handles.into_reverse().collect())?;

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
        let (output, outputs) = futures::try_join!(self.head.err_into(), self.tail.collect())?;

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
