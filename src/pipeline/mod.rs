pub(crate) mod action_operator;
pub(crate) mod pull_connector;
pub(crate) mod push_connector;
mod sensor_operator;
mod state_committer;
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
    state_store::{ProgressedOutput, Session},
};
use crate::{context::Context, package::prepared::PreparedPackage};

pub(crate) struct Pipeline<Item, Si, Handles> {
    sink: Si,
    handles: Handles,

    session: Session,

    _marker: PhantomData<Item>,
}

impl<Item> Pipeline<Item, sink::SinkErrInto<sink::Drain<Item>, Item, anyhow::Error>, HNil> {
    pub(crate) fn build(
        prepared_package: PreparedPackage,
        pb: ProgressBar,
        context: Arc<Context>,
    ) -> Self {
        let sink = sink::drain();
        let sink = sink.sink_err_into();

        let session = Session::new(prepared_package, pb, context);

        Self {
            sink,
            handles: HNil,

            session,

            _marker: PhantomData,
        }
    }
}

impl<
    Item: Clone + Send,
    Si: sink::Sink<Item, Error = anyhow::Error> + Send + 'static,
    Handles: IntoReverse<Output: Collect>,
> Pipeline<Item, Si, Handles>
{
    #[expect(clippy::type_complexity)]
    pub(crate) fn with_progressor<_ConnMarker>(
        self,
        content_length: Option<u64>,
    ) -> anyhow::Result<
        Pipeline<
            Item,
            sink::Fanout<Si, <Progressor as Connector<Item, _ConnMarker>>::Sink>,
            HCons<AbortOnDropHandle<anyhow::Result<ProgressedOutput>>, Handles>,
        >,
    >
    where
        Progressor: Connector<
                Item,
                _ConnMarker,
                Sink: sink::Sink<Item, Error = anyhow::Error>,
                Output = ProgressedOutput,
            >,
    {
        let pb = &self.session.pb;

        let progressor = Progressor::try_new(pb.clone(), content_length)?;

        let pipeline = self.fanout(progressor);

        Ok(pipeline)
    }

    #[expect(clippy::type_complexity)]
    pub(crate) fn fanout<
        Conn: Connector<Item, _ConnMarker, Sink: sink::Sink<Item, Error = anyhow::Error>>,
        _ConnMarker,
    >(
        self,
        connector: Conn,
    ) -> Pipeline<
        Item,
        sink::Fanout<Si, Conn::Sink>,
        HCons<AbortOnDropHandle<anyhow::Result<Conn::Output>>, Handles>,
    > {
        let (sink, handle) = connector.launch(self.session.clone());

        let sink = self.sink.fanout(sink);

        Pipeline {
            sink,
            handles: HCons {
                head: handle,
                tail: self.handles,
            },

            session: self.session,

            _marker: PhantomData,
        }
    }

    pub(crate) async fn run_concurrently(
        self,
        stream: impl stream::TryStream<Ok = Item, Error = anyhow::Error> + Send + 'static,
    ) -> anyhow::Result<HCons<Arc<PreparedPackage>, <Handles::Output as Collect>::Outputs>> {
        let handle = task::spawn(async {
            let stream = stream.err_into();

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

        let prepared_package = self.session.prepared_package;

        let output = HCons {
            head: prepared_package,
            tail: outputs,
        };

        Ok(output)
    }
}

pub(crate) trait Connector<Item, _ConnMarker>: Sized {
    type Sink;
    type Output;

    fn launch(
        self,
        session: Session,
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

pub(crate) trait Operator<_OpMarker> {
    type Input;
    type Output;

    fn proceed(
        self,
        input: Self::Input,
        session: Session,
    ) -> AbortOnDropHandle<anyhow::Result<Self::Output>>;
}

pub(crate) trait Operators<Input, _OpMarker> {
    type Handles;

    fn proceed_all(self, input: Input, session: Session) -> Self::Handles;
}

impl<Input, _OpMarker> Operators<Input, _OpMarker> for HNil {
    type Handles = Self;

    fn proceed_all(self, _input: Input, _session: Session) -> Self::Handles {
        Self
    }
}

impl<
    Input: Clone,
    Op: Operator<_OpMarker, Input = Input>,
    Ops: Operators<Input, _OpMarker>,
    _OpMarker,
> Operators<Input, _OpMarker> for HCons<Op, Ops>
{
    type Handles = HCons<AbortOnDropHandle<anyhow::Result<Op::Output>>, Ops::Handles>;

    fn proceed_all(self, input: Input, session: Session) -> Self::Handles {
        let handle = self.head.proceed(input.clone(), session.clone());

        let handles = self.tail.proceed_all(input, session);

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
trait ReversedCollect = IntoReverse<Output: Collect<Outputs: Send + 'static>>;

#[cfg(not(debug_assertions))]
trait ReversedCollect: IntoReverse<Output: Collect<Outputs: Send + 'static>> {}

#[cfg(not(debug_assertions))]
impl<Handles: IntoReverse<Output: Collect<Outputs: Send + 'static>>> ReversedCollect for Handles {}

impl<
    Item,
    Conn: Connector<Item, _ConnMarker, Output: Clone + Send + 'static>,
    Ops: Operators<Conn::Output, _OpMarker, Handles: ReversedCollect> + Send + 'static,
    _ConnMarker,
    _OpMarker,
> Connector<Item, (_ConnMarker, _OpMarker)> for FanoutOperator<Conn, Ops>
{
    type Sink = Conn::Sink;
    type Output = HCons<Conn::Output, <<Ops::Handles as IntoReverse>::Output as Collect>::Outputs>;

    fn launch(
        self,
        session: Session,
    ) -> (Self::Sink, AbortOnDropHandle<anyhow::Result<Self::Output>>) {
        let (sink, handle) = self.connector.launch(session.clone());

        let handle = task::spawn(async {
            let connector_output = handle.await??;

            let operators_input = connector_output.clone();

            let handles = self.operators.proceed_all(operators_input, session);
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
