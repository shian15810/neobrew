pub(crate) mod action_operator;
pub(crate) mod pull_connector;
pub(crate) mod push_connector;
pub(crate) mod sensor_operator;
mod state_committer;
mod state_store;

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
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
    push_connector::progressor::Progressor,
    state_store::{ProgressedOutput, Session},
};
use crate::{
    context::Context,
    package::prepared::{PreparedPackage, download::Download},
};

pub(crate) struct Pipeline<Si, Handles> {
    sink: Si,
    handles: Handles,

    session: Session,
}

impl Pipeline<sink::SinkErrInto<sink::Drain<Bytes>, Bytes, anyhow::Error>, HNil> {
    pub(crate) fn build(
        prepared_package: PreparedPackage<Download>,
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
        }
    }
}

impl<
    Si: sink::Sink<Bytes, Error = anyhow::Error> + Send + 'static,
    Handles: IntoReverse<Output: Collect>,
> Pipeline<Si, Handles>
{
    #[expect(clippy::type_complexity)]
    pub(crate) fn fanout<
        Conn: Connector<_ConnMarker, Sink: sink::Sink<Bytes, Error = anyhow::Error>>,
        _ConnMarker,
    >(
        self,
        connector: Conn,
    ) -> Pipeline<
        sink::Fanout<Si, Conn::Sink>,
        HCons<AbortOnDropHandle<anyhow::Result<Option<Conn::Output>>>, Handles>,
    > {
        let session = self.session.clone();

        let (sink, handle) = connector.launch(session);

        let sink = self.sink.fanout(sink);

        Pipeline {
            sink,
            handles: HCons {
                head: handle,
                tail: self.handles,
            },

            session: self.session,
        }
    }

    #[expect(clippy::type_complexity)]
    pub(crate) fn with_pb<_ConnMarker>(
        self,
    ) -> Pipeline<
        sink::Fanout<Si, <Progressor as Connector<_ConnMarker>>::Sink>,
        HCons<AbortOnDropHandle<anyhow::Result<Option<ProgressedOutput>>>, Handles>,
    >
    where
        Progressor: Connector<
                _ConnMarker,
                Sink: sink::Sink<Bytes, Error = anyhow::Error>,
                Output = ProgressedOutput,
            >,
    {
        let pb = &self.session.pb;

        let progressor = Progressor::new(pb.clone());

        self.fanout(progressor)
    }

    pub(crate) async fn run_concurrently(
        self,
        stream: impl stream::TryStream<Ok = Bytes, Error = anyhow::Error> + Send + 'static,
    ) -> anyhow::Result<<Handles::Output as Collect>::Outputs> {
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

        Ok(outputs)
    }
}

#[expect(clippy::type_complexity)]
pub(crate) trait Connector<_ConnMarker>: Sized {
    type Sink;
    type Output;

    fn launch(
        self,
        session: Session,
    ) -> (
        Self::Sink,
        AbortOnDropHandle<anyhow::Result<Option<Self::Output>>>,
    );

    fn fanout<Op>(self, operator: Op) -> Fanout<Self, HCons<Op, HNil>> {
        Fanout {
            source: self,
            operators: HCons {
                head: operator,
                tail: HNil,
            },
        }
    }
}

pub(crate) trait Operator<_OpMarker>: Sized {
    type Input;
    type Output;

    fn proceed(
        self,
        input: Option<Self::Input>,
        session: Session,
    ) -> AbortOnDropHandle<anyhow::Result<Option<Self::Output>>>;

    fn fanout<Op>(self, operator: Op) -> Fanout<Self, HCons<Op, HNil>> {
        Fanout {
            source: self,
            operators: HCons {
                head: operator,
                tail: HNil,
            },
        }
    }
}

pub(crate) trait Operators<Input, _OpsMarker> {
    type Handles;

    fn proceed_all(self, input: Option<Input>, session: Session) -> Self::Handles;
}

impl<Input> Operators<Input, ()> for HNil {
    type Handles = Self;

    fn proceed_all(self, _input: Option<Input>, _session: Session) -> Self::Handles {
        Self
    }
}

impl<
    Input: Clone,
    Op: Operator<_OpMarker, Input = Input>,
    Ops: Operators<Input, _OpsMarker>,
    _OpMarker,
    _OpsMarker,
> Operators<Input, (_OpMarker, _OpsMarker)> for HCons<Op, Ops>
{
    type Handles = HCons<AbortOnDropHandle<anyhow::Result<Option<Op::Output>>>, Ops::Handles>;

    fn proceed_all(self, input: Option<Input>, session: Session) -> Self::Handles {
        let handle = self.head.proceed(input.clone(), session.clone());

        let handles = self.tail.proceed_all(input, session);

        HCons {
            head: handle,
            tail: handles,
        }
    }
}

pub(crate) struct Fanout<Src, Ops> {
    source: Src,
    operators: Ops,
}

impl<Src, Ops> Fanout<Src, Ops> {
    pub(crate) fn fanout<Op>(self, operator: Op) -> Fanout<Src, HCons<Op, Ops>> {
        Fanout {
            source: self.source,
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
    Conn: Connector<_ConnMarker, Output: Clone + Send + 'static>,
    Ops: Operators<Conn::Output, _OpsMarker, Handles: ReversedCollect> + Send + 'static,
    _ConnMarker,
    _OpsMarker,
> Connector<(_ConnMarker, _OpsMarker)> for Fanout<Conn, Ops>
{
    type Sink = Conn::Sink;
    type Output =
        HCons<Option<Conn::Output>, <<Ops::Handles as IntoReverse>::Output as Collect>::Outputs>;

    fn launch(
        self,
        session: Session,
    ) -> (
        Self::Sink,
        AbortOnDropHandle<anyhow::Result<Option<Self::Output>>>,
    ) {
        let (sink, handle) = self.source.launch(session.clone());

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

            anyhow::Ok(Some(output))
        });
        let handle = AbortOnDropHandle::new(handle);

        (sink, handle)
    }
}

impl<
    Op: Operator<_OpMarker, Output: Clone + Send + 'static>,
    Ops: Operators<Op::Output, _OpsMarker, Handles: ReversedCollect> + Send + 'static,
    _OpMarker,
    _OpsMarker,
> Operator<(_OpMarker, _OpsMarker)> for Fanout<Op, Ops>
{
    type Input = Op::Input;
    type Output =
        HCons<Option<Op::Output>, <<Ops::Handles as IntoReverse>::Output as Collect>::Outputs>;

    #[expect(clippy::let_and_return)]
    fn proceed(
        self,
        input: Option<Self::Input>,
        session: Session,
    ) -> AbortOnDropHandle<anyhow::Result<Option<Self::Output>>> {
        let handle = self.source.proceed(input, session.clone());

        let handle = task::spawn(async {
            let operator_output = handle.await??;

            let operators_input = operator_output.clone();

            let handles = self.operators.proceed_all(operators_input, session);
            let handles = handles.into_reverse();

            let operators_outputs = handles.collect().await?;

            let output = HCons {
                head: operator_output,
                tail: operators_outputs,
            };

            anyhow::Ok(Some(output))
        });
        let handle = AbortOnDropHandle::new(handle);

        handle
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
impl<Output: Send, Handles: Collect<Outputs: Send> + Send> Collect
    for HCons<AbortOnDropHandle<anyhow::Result<Output>>, Handles>
{
    type Outputs = HCons<Output, Handles::Outputs>;

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
