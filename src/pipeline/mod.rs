use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use anyhow::{Error, Result};
use futures::{
    sink::{self, SinkExt},
    stream::{self, StreamExt, TryStreamExt},
};
use tokio::{sync::oneshot, task::JoinSet};
use tokio_util::sync::PollSender;

use crate::context::Context;

pub mod pipe_operators;
pub mod tee_operators;

pub trait Operator<Item, _Marker> {
    type Output;

    fn spawn(
        self,
        set: &mut JoinSet<Result<()>>,
        context: &Context,
    ) -> (PollSender<Item>, oneshot::Receiver<Self::Output>);
}

pub struct Pipeline<Item, St, Si, Receivers> {
    stream: St,
    sink: Si,
    output_rxs: Receivers,

    set: JoinSet<Result<()>>,

    context: Arc<Context>,

    _marker: PhantomData<Item>,
}

impl<Item, St> Pipeline<Item, St, sink::SinkErrInto<sink::Drain<Item>, Item, Error>, ()> {
    pub fn new(stream: St, context: Arc<Context>) -> Self {
        Self {
            stream,
            sink: sink::drain().sink_err_into(),
            output_rxs: (),

            set: JoinSet::new(),

            context,

            _marker: PhantomData,
        }
    }
}

impl<
    Item: Clone + Debug + Send + Sync + 'static,
    St: stream::TryStream<Ok = Item, Error = impl Into<Error>> + Send + 'static,
    Si: sink::Sink<Item, Error = Error> + Send + 'static,
    Receivers: Collect,
> Pipeline<Item, St, Si, Receivers>
{
    pub fn fanout<_Marker, Op: Operator<Item, _Marker>>(
        mut self,
        operator: Op,
    ) -> Pipeline<
        Item,
        St,
        sink::Fanout<Si, sink::SinkErrInto<PollSender<Item>, Item, Error>>,
        (Receivers, oneshot::Receiver<Op::Output>),
    > {
        let (sink, output_rx) = operator.spawn(&mut self.set, &self.context);

        Pipeline {
            stream: self.stream,
            sink: self.sink.fanout(sink.sink_err_into()),
            output_rxs: (self.output_rxs, output_rx),

            set: self.set,

            context: self.context,

            _marker: PhantomData,
        }
    }

    pub async fn spawn(mut self) -> Result<<Receivers::Output as Flatten>::Output>
    where
        Receivers::Output: Flatten,
    {
        self.set.spawn(async move {
            self.stream.err_into().forward(self.sink).await?;

            Ok(())
        });

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
