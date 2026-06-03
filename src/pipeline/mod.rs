mod channels;
pub(crate) mod post_operator;
pub(crate) mod pull_operator;
pub(crate) mod push_operator;

use std::{fmt::Debug, marker::PhantomData, sync::Arc};

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

use self::{channels::PipelineChannels as Channels, post_operator::PostOperator};
use crate::context::Context;

pub(crate) struct Pipeline<Item, St, Si, Handles> {
    stream: St,
    sink: Si,
    handles: Handles,

    channels: Arc<Channels>,

    context: Arc<Context>,

    _marker: PhantomData<Item>,
}

impl<Item, St> Pipeline<Item, St, sink::SinkErrInto<sink::Drain<Item>, Item, anyhow::Error>, HNil> {
    pub(crate) fn build(stream: St, context: Arc<Context>) -> Self {
        let sink = sink::drain();
        let sink = sink.sink_err_into();

        let channels = Channels::new();
        let channels = Arc::new(channels);

        Self {
            stream,
            sink,
            handles: HNil,

            channels,

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
        HCons<AbortOnDropHandle<anyhow::Result<Op::Output>>, Handles>,
    > {
        let (sink, handle) = operator.launch(Arc::clone(&self.channels), Arc::clone(&self.context));

        let sink = sink.sink_err_into();
        let sink = self.sink.fanout(sink);

        Pipeline {
            stream: self.stream,
            sink,
            handles: HCons {
                head: handle,
                tail: self.handles,
            },

            channels: self.channels,

            context: self.context,

            _marker: PhantomData,
        }
    }

    pub(crate) async fn run_parallel(
        self,
    ) -> anyhow::Result<<Handles::Output as Collect>::Outputs> {
        let handle = task::spawn(async {
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

    async fn collect(self) -> anyhow::Result<Self::Outputs>;
}

impl Collect for HNil {
    type Outputs = Self;

    #[expect(clippy::unused_async_trait_impl)]
    async fn collect(self) -> anyhow::Result<Self::Outputs> {
        let outputs = Self;

        Ok(outputs)
    }
}

impl<Item, Handles: Collect> Collect for HCons<AbortOnDropHandle<anyhow::Result<Item>>, Handles> {
    type Outputs = HCons<Item, Handles::Outputs>;

    async fn collect(self) -> anyhow::Result<Self::Outputs> {
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

pub(crate) trait Operator<Item, _Marker>: Sized {
    type Output;

    fn launch(
        self,
        channels: Arc<Channels>,
        context: Arc<Context>,
    ) -> (
        PollSender<Item>,
        AbortOnDropHandle<anyhow::Result<Self::Output>>,
    );

    fn pipe<PostOp>(self, post_operator: PostOp) -> PipedOperator<Self, PostOp> {
        PipedOperator {
            operator: self,
            post_operator,
        }
    }
}

pub(crate) struct PipedOperator<Op, PostOp> {
    operator: Op,
    post_operator: PostOp,
}

impl<
    Item,
    Op: Operator<Item, _Marker, Output: Send + 'static>,
    PostOp: PostOperator<Input = Op::Output, Output: Send> + 'static,
    _Marker,
> Operator<Item, _Marker> for PipedOperator<Op, PostOp>
{
    type Output = PostOp::Output;

    fn launch(
        self,
        channels: Arc<Channels>,
        context: Arc<Context>,
    ) -> (
        PollSender<Item>,
        AbortOnDropHandle<anyhow::Result<Self::Output>>,
    ) {
        let (sink, handle) = self
            .operator
            .launch(Arc::clone(&channels), Arc::clone(&context));

        let handle = task::spawn(async {
            let input = handle.await??;

            let output = self.post_operator.proceed(input, channels, context).await?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}
