use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use anyhow::{Error, Result};
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

    fn spawn(
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

impl<Item, St> Pipeline<Item, St, sink::SinkErrInto<sink::Drain<Item>, Item, Error>, ()> {
    pub fn new(stream: St, context: Arc<Context>) -> Self {
        Self {
            stream,
            sink: sink::drain().sink_err_into(),
            handles: (),

            context,

            _marker: PhantomData,
        }
    }
}

impl<
    Item: Clone + Debug + Send + Sync + 'static,
    St: stream::TryStream<Ok = Item, Error = impl Into<Error>> + Send + 'static,
    Si: sink::Sink<Item, Error = Error> + Send + 'static,
    Handles: TryJoin,
> Pipeline<Item, St, Si, Handles>
{
    pub fn fanout<_Marker, Op: Operator<Item, _Marker>>(
        self,
        operator: Op,
    ) -> Pipeline<
        Item,
        St,
        sink::Fanout<Si, sink::SinkErrInto<PollSender<Item>, Item, Error>>,
        (Handles, AbortOnDropHandle<Result<Op::Output>>),
    > {
        let (sink, handle) = operator.spawn(&self.context);

        Pipeline {
            stream: self.stream,
            sink: self.sink.fanout(sink.sink_err_into()),
            handles: (self.handles, handle),

            context: self.context,

            _marker: PhantomData,
        }
    }

    pub async fn spawn(self) -> Result<<Handles::Output as Flatten>::Output>
    where
        Handles::Output: Flatten,
    {
        let handle = task::spawn(self.stream.err_into().forward(self.sink));
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        let outputs = self.handles.try_join().await?;
        let outputs = outputs.flatten();

        Ok(outputs)
    }
}

// --- TryJoin ---

pub trait TryJoin {
    type Output;

    async fn try_join(self) -> Result<Self::Output>;
}

impl TryJoin for () {
    type Output = ();

    async fn try_join(self) -> Result<Self::Output> {
        Ok(())
    }
}

impl<Handles: TryJoin, Output> TryJoin for (Handles, AbortOnDropHandle<Result<Output>>) {
    type Output = (Handles::Output, Output);

    async fn try_join(self) -> Result<Self::Output> {
        let (handles, handle) = self;

        let outputs = handles.try_join().await?;

        let output = handle.await??;

        let outputs = (outputs, output);

        Ok(outputs)
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

impl<Handles: Flatten, Output> Flatten for (Handles, Output)
where
    Handles::Output: Append<Output>,
{
    type Output = <Handles::Output as Append<Output>>::Output;

    fn flatten(self) -> Self::Output {
        let (handles, output) = self;

        let outputs = handles.flatten();

        outputs.append(output)
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
