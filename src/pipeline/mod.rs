use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use anyhow::{Error, Result};
use futures::{
    sink::{self, SinkExt},
    stream::{self, StreamExt, TryStreamExt},
};
use tokio::{sync::oneshot, task::JoinSet};

use self::operator::{BlockingSink, Operator};
use crate::context::Context;

pub mod operator;

type DrainSink<Item> = sink::SinkErrInto<sink::Drain<Item>, Item, Error>;
type FanoutSink<Item, Si> = sink::Fanout<Si, sink::SinkErrInto<BlockingSink<Item>, Item, Error>>;

pub struct Pipeline<Item, Si, Receivers> {
    _marker: PhantomData<Item>,

    sink: Si,
    receivers: Receivers,

    set: JoinSet<Result<()>>,

    context: Arc<Context>,
}

impl<Item> Pipeline<Item, DrainSink<Item>, ()> {
    pub fn new(context: Arc<Context>) -> Self {
        Self {
            _marker: PhantomData,

            sink: sink::drain().sink_err_into(),
            receivers: (),

            set: JoinSet::new(),

            context,
        }
    }
}

impl<
    Item: Clone + Debug + Send + Sync + 'static,
    Si: sink::Sink<Item, Error = Error>,
    Receivers: Collect,
> Pipeline<Item, Si, Receivers>
where
    Receivers::Output: Flatten,
{
    pub fn fanout<Output: Send + 'static>(
        mut self,
        operator: impl Operator<Item, Output>,
    ) -> Pipeline<Item, FanoutSink<Item, Si>, (Receivers, oneshot::Receiver<Output>)> {
        let (sink, output_rx) = operator.spawn(&mut self.set, &self.context);

        Pipeline {
            _marker: PhantomData,

            sink: self.sink.fanout(sink.sink_err_into()),
            receivers: (self.receivers, output_rx),

            set: self.set,

            context: self.context,
        }
    }

    pub async fn send_all<E: Into<Error>>(
        mut self,
        stream: impl stream::Stream<Item = Result<Item, E>>,
    ) -> Result<<Receivers::Output as Flatten>::Output> {
        stream.err_into().forward(self.sink).await?;

        while let Some(res) = self.set.join_next().await {
            res??;
        }

        let outputs = self.receivers.collect().await?;

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
        let (receivers, receiver) = self;

        let outputs = receivers.collect().await?;

        let output = receiver.await?;

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
        let (receivers, output) = self;

        receivers.flatten().append(output)
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
