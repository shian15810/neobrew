use std::{fmt::Debug, io, sync::Arc};

use anyhow::{Error, Result};
use futures::{
    sink::{self, SinkExt},
    stream::{self, StreamExt, TryStreamExt},
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};

use self::{
    pipe_operators::PipeOperator,
    tee_operators::{BlockingSink, TeeOperator},
};
use crate::context::Context;

pub mod pipe_operators;
pub mod tee_operators;

type DrainSink<Item> = sink::SinkErrInto<sink::Drain<Item>, Item, Error>;
type FanoutSink<Item, Si> = sink::Fanout<Si, sink::SinkErrInto<BlockingSink<Item>, Item, Error>>;

pub trait Operator<InputHandle, Output> {
    fn spawn(
        self,
        set: &mut JoinSet<Result<()>>,
        context: &Context,
    ) -> (InputHandle, oneshot::Receiver<Output>);
}

pub struct Pipeline<Item, St, Si, Receivers> {
    stream: St,

    input_txs: Vec<mpsc::Sender<io::Result<Item>>>,
    sink: Si,

    output_rxs: Receivers,

    set: JoinSet<Result<()>>,

    context: Arc<Context>,
}

impl<Item, St> Pipeline<Item, St, DrainSink<Item>, ()> {
    pub fn new(stream: St, context: Arc<Context>) -> Self {
        Self {
            stream,

            input_txs: Vec::new(),
            sink: sink::drain().sink_err_into(),

            output_rxs: (),

            set: JoinSet::new(),

            context,
        }
    }
}

impl<
    Item: Clone + Debug + Send + Sync + 'static,
    St: stream::Stream<Item = Result<Item, impl Into<Error>>>,
    Si: sink::Sink<Item, Error = Error>,
    Receivers: Collect,
> Pipeline<Item, St, Si, Receivers>
where
    Receivers::Output: Flatten,
{
    pub fn forward<Output>(
        mut self,
        pipe_operator: impl PipeOperator<Output> + Operator<mpsc::Sender<io::Result<Item>>, Output>,
    ) -> Pipeline<Item, St, Si, (Receivers, oneshot::Receiver<Output>)> {
        let (input_tx, output_rx) = pipe_operator.spawn(&mut self.set, &self.context);

        self.input_txs.push(input_tx);

        Pipeline {
            stream: self.stream,

            input_txs: self.input_txs,
            sink: self.sink,

            output_rxs: (self.output_rxs, output_rx),

            set: self.set,

            context: self.context,
        }
    }

    pub fn fanout<Output>(
        mut self,
        tee_operator: impl TeeOperator<Item, Output> + Operator<BlockingSink<Item>, Output>,
    ) -> Pipeline<Item, St, FanoutSink<Item, Si>, (Receivers, oneshot::Receiver<Output>)> {
        let (sink, output_rx) = tee_operator.spawn(&mut self.set, &self.context);

        Pipeline {
            stream: self.stream,

            input_txs: self.input_txs,
            sink: self.sink.fanout(sink.sink_err_into()),

            output_rxs: (self.output_rxs, output_rx),

            set: self.set,

            context: self.context,
        }
    }

    pub async fn spawn(mut self) -> Result<<Receivers::Output as Flatten>::Output> {
        self.stream.err_into().forward(self.sink).await?;

        while let Some(res) = self.set.join_next().await {
            res??;
        }

        let outputs = self.output_rxs.collect().await?;

        Ok(outputs.flatten())
    }
}

// --- Collect ---

pub trait Collect {
    type Output;

    async fn collect(self) -> Result<Self::Output>;
}

impl Collect for () {
    type Output = ();

    async fn collect(self) -> Result<Self::Output> {
        Ok(())
    }
}

impl<Receivers: Collect, Output> Collect for (Receivers, oneshot::Receiver<Output>) {
    type Output = (Receivers::Output, Output);

    async fn collect(self) -> Result<Self::Output> {
        let (output_rxs, output_rx) = self;

        let outputs = output_rxs.collect().await?;

        let output = output_rx.await?;

        Ok((outputs, output))
    }
}

// --- Flatten ---

pub trait Flatten {
    type Output;

    fn flatten(self) -> Self::Output;
}

impl Flatten for () {
    type Output = ();

    fn flatten(self) -> Self::Output {}
}

impl<Receivers: Flatten, Output> Flatten for (Receivers, Output)
where
    Receivers::Output: Append<Output>,
{
    type Output = <Receivers::Output as Append<Output>>::Output;

    fn flatten(self) -> Self::Output {
        let (output_rxs, output) = self;

        output_rxs.flatten().append(output)
    }
}

// --- Append ---

pub trait Append<Output> {
    type Output;

    fn append(self, output: Output) -> Self::Output;
}

macro_rules! impl_append {
    () => {
        impl<Output> Append<Output> for () {
            type Output = (Output,);

            fn append(self, output: Output) -> Self::Output {
                (output,)
            }
        }
    };

    ($head:ident $(, $tail:ident)*) => {
        impl<Output, $head, $($tail,)*> Append<Output> for ($head, $($tail,)*) {
            type Output = ($head, $($tail,)* Output);

            fn append(self, output: Output) -> Self::Output {
                #[allow(non_snake_case)]
                let ($head, $($tail,)*) = self;

                ($head, $($tail,)* output)
            }
        }

        impl_append!($($tail),*);
    };
}

impl_append!(
    T15, T14, T13, T12, T11, T10, T9, T8, T7, T6, T5, T4, T3, T2, T1, T0
);
