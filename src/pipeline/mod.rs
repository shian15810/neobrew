use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use anyhow::{Error, Result};
use frunk::hlist::{HCons, HNil};
use futures::{
    sink::{self, SinkExt},
    stream::{self, StreamExt, TryStreamExt},
};
use tokio::task;
use tokio_util::{sync::PollSender, task::AbortOnDropHandle};

use crate::context::Context;

pub mod pull_operators;
pub mod push_operators;

pub trait Operator<Item, _Marker> {
    type Output;

    fn spawn_blocking(
        self,
        context: &Context,
    ) -> (PollSender<Item>, AbortOnDropHandle<Result<Self::Output>>);
}

pub struct Pipeline<Item, St, Si, Handles> {
    stream: St,
    sink: Si,
    handles: Handles,

    context: Arc<Context>,

    _marker: PhantomData<Item>,
}

impl<Item, St> Pipeline<Item, St, sink::SinkErrInto<sink::Drain<Item>, Item, Error>, HNil> {
    pub fn new(stream: St, context: Arc<Context>) -> Self {
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
    Handles: Collect,
> Pipeline<Item, St, Si, Handles>
{
    pub fn fanout<Op: Operator<Item, _Marker>, _Marker>(
        self,
        operator: Op,
    ) -> Pipeline<
        Item,
        St,
        sink::Fanout<Si, sink::SinkErrInto<PollSender<Item>, Item, Error>>,
        HCons<AbortOnDropHandle<Result<Op::Output>>, Handles>,
    > {
        let (sink, handle) = operator.spawn_blocking(&self.context);

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

    pub async fn spawn(self) -> Result<Handles::Outputs> {
        let handle = task::spawn(async move {
            let forward = self.stream.err_into().forward(self.sink);

            forward.await
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        let outputs = self.handles.collect().await?;

        Ok(outputs)
    }
}

pub trait Collect {
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
        let outputs = HCons {
            head: self.head.await??,
            tail: self.tail.collect().await?,
        };

        Ok(outputs)
    }
}
