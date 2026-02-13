use anyhow::Result;
use bytes::Bytes;

pub use self::{hasher::Hasher, writer::Writer};

mod hasher;
mod writer;

pub trait Operator: Send + 'static {
    fn send(&mut self, chunk: Bytes) -> Result<()>;

    fn apply(self) -> Result<()>;
}
