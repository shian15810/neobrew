pub(crate) mod action_operator;
pub(crate) mod pull_connector;
pub(crate) mod push_connector;
mod sensor_operator;
mod state_store;

use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use frunk::{
    hlist::{HCons, HNil},
    traits::IntoReverse,
};
use futures::{
    future::TryFutureExt as _,
    sink::{self, SinkExt as _},
    stream::{self, StreamExt as _, TryStreamExt as _},
};
use indicatif::ProgressBar;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

use self::{
    push_connector::Progressor,
    state_store::{Channel, ProgressedOutput},
};
use crate::context::Context;

pub(crate) struct Pipeline<Item, St, Si, Handles> {
    stream: St,
    sink: Si,
    handles: Handles,

    pb: ProgressBar,
    channel: Channel,
    context: Arc<Context>,

    _marker: PhantomData<Item>,
}

impl<Item, St> Pipeline<Item, St, sink::SinkErrInto<sink::Drain<Item>, Item, anyhow::Error>, HNil> {
    pub(crate) fn build(stream: St, pb: ProgressBar, context: Arc<Context>) -> Self {
        let sink = sink::drain();
        let sink = sink.sink_err_into();

        let channel = Channel::new();

        Self {
            stream,
            sink,
            handles: HNil,

            pb,
            channel,
            context,

            _marker: PhantomData,
        }
    }
}

impl<
    Item: Clone + Send,
    St: stream::TryStream<Ok = Item, Error = anyhow::Error> + Send + 'static,
    Si: sink::Sink<Item, Error = anyhow::Error> + Send + 'static,
    Handles: IntoReverse<Output: Collect>,
> Pipeline<Item, St, Si, Handles>
{
    #[expect(clippy::type_complexity)]
    pub(crate) fn with_progressor<_Marker>(
        self,
        content_length: Option<u64>,
    ) -> anyhow::Result<
        Pipeline<
            Item,
            St,
            sink::Fanout<Si, <Progressor as Connector<Item, _Marker>>::Sink>,
            HCons<AbortOnDropHandle<anyhow::Result<ProgressedOutput>>, Handles>,
        >,
    >
    where
        Progressor: Connector<
                Item,
                _Marker,
                Sink: sink::Sink<Item, Error = anyhow::Error>,
                Output = ProgressedOutput,
            >,
    {
        let progressor = Progressor::try_new(self.pb.clone(), content_length)?;

        let pipeline = self.fanout(progressor);

        Ok(pipeline)
    }

    #[expect(clippy::type_complexity)]
    pub(crate) fn fanout<
        Conn: Connector<Item, _Marker, Sink: sink::Sink<Item, Error = anyhow::Error>>,
        _Marker,
    >(
        self,
        connector: Conn,
    ) -> Pipeline<
        Item,
        St,
        sink::Fanout<Si, Conn::Sink>,
        HCons<AbortOnDropHandle<anyhow::Result<Conn::Output>>, Handles>,
    > {
        let (sink, handle) = connector.launch(
            self.pb.clone(),
            self.channel.clone(),
            Arc::clone(&self.context),
        );

        let sink = self.sink.fanout(sink);

        Pipeline {
            stream: self.stream,
            sink,
            handles: HCons {
                head: handle,
                tail: self.handles,
            },

            pb: self.pb,
            channel: self.channel,
            context: self.context,

            _marker: PhantomData,
        }
    }

    pub(crate) async fn run_concurrently(
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

pub(crate) trait Connector<Item, _Marker>: Sized {
    type Sink;
    type Output;

    fn launch(
        self,
        pb: ProgressBar,
        channel: Channel,
        context: Arc<Context>,
    ) -> (Self::Sink, AbortOnDropHandle<anyhow::Result<Self::Output>>);

    fn fanout<Op>(self, operator: Op) -> FanoutOperator<Self, HCons<Op, HNil>> {
        FanoutOperator {
            connector: self,
            operators: HCons {
                head: operator,
                tail: HNil,
            },
        }
    }
}

pub(crate) trait Operator {
    type Input;
    type Output;

    fn proceed(
        self,
        input: Self::Input,
        pb: ProgressBar,
        channel: Channel,
        context: Arc<Context>,
    ) -> AbortOnDropHandle<anyhow::Result<Self::Output>>;
}

pub(crate) trait Operators<Input> {
    type Handles;

    fn proceed_all(
        self,
        input: Input,
        pb: ProgressBar,
        channel: Channel,
        context: Arc<Context>,
    ) -> Self::Handles;
}

impl<Input> Operators<Input> for HNil {
    type Handles = Self;

    fn proceed_all(
        self,
        _input: Input,
        _pb: ProgressBar,
        _channel: Channel,
        _context: Arc<Context>,
    ) -> Self::Handles {
        Self
    }
}

impl<Input: Clone, Op: Operator<Input = Input>, Ops: Operators<Input>> Operators<Input>
    for HCons<Op, Ops>
{
    type Handles = HCons<AbortOnDropHandle<anyhow::Result<Op::Output>>, Ops::Handles>;

    fn proceed_all(
        self,
        input: Input,
        pb: ProgressBar,
        channel: Channel,
        context: Arc<Context>,
    ) -> Self::Handles {
        let handle = self.head.proceed(
            input.clone(),
            pb.clone(),
            channel.clone(),
            Arc::clone(&context),
        );

        let handles = self.tail.proceed_all(input, pb, channel, context);

        HCons {
            head: handle,
            tail: handles,
        }
    }
}

pub(crate) struct FanoutOperator<Conn, Ops> {
    connector: Conn,
    operators: Ops,
}

impl<Conn, Ops> FanoutOperator<Conn, Ops> {
    fn fanout<Op>(self, operator: Op) -> FanoutOperator<Conn, HCons<Op, Ops>> {
        FanoutOperator {
            connector: self.connector,
            operators: HCons {
                head: operator,
                tail: self.operators,
            },
        }
    }
}

#[cfg(debug_assertions)]
trait ReversibleCollect = IntoReverse<Output: Collect<Outputs: Send>>;

#[cfg(not(debug_assertions))]
trait ReversibleCollect: IntoReverse<Output: Collect<Outputs: Send>> {}

#[cfg(not(debug_assertions))]
impl<Handles: IntoReverse<Output: Collect<Outputs: Send>>> ReversibleCollect for Handles {}

impl<
    Item,
    Conn: Connector<Item, _Marker, Output: Clone + Send + 'static>,
    Ops: Operators<Conn::Output, Handles: ReversibleCollect> + Send + 'static,
    _Marker,
> Connector<Item, _Marker> for FanoutOperator<Conn, Ops>
{
    type Sink = Conn::Sink;
    type Output = HCons<Conn::Output, <<Ops::Handles as IntoReverse>::Output as Collect>::Outputs>;

    fn launch(
        self,
        pb: ProgressBar,
        channel: Channel,
        context: Arc<Context>,
    ) -> (Self::Sink, AbortOnDropHandle<anyhow::Result<Self::Output>>) {
        let (sink, handle) =
            self.connector
                .launch(pb.clone(), channel.clone(), Arc::clone(&context));

        let handle = task::spawn(async {
            let connector_output = handle.await??;

            let operators_input = connector_output.clone();

            let handles = self
                .operators
                .proceed_all(operators_input, pb, channel, context);
            let handles = handles.into_reverse();

            let operators_outputs = handles.collect().await?;

            let output = HCons {
                head: connector_output,
                tail: operators_outputs,
            };

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}

#[async_trait]
pub(crate) trait Collect {
    type Outputs;

    async fn collect(self) -> anyhow::Result<Self::Outputs>;
}

#[async_trait]
impl Collect for HNil {
    type Outputs = Self;

    async fn collect(self) -> anyhow::Result<Self::Outputs> {
        let outputs = Self;

        Ok(outputs)
    }
}

#[async_trait]
impl<Item: Send, Handles: Collect<Outputs: Send> + Send> Collect
    for HCons<AbortOnDropHandle<anyhow::Result<Item>>, Handles>
{
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
