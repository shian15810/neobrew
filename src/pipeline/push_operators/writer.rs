use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::Result;
use bytes::Bytes;

use super::PushOperator;

pub struct Writer {
    inner: BufWriter<File>,
}

impl Writer {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let this = Self {
            inner: BufWriter::new(File::create(path)?),
        };

        Ok(this)
    }
}

impl PushOperator for Writer {
    type Item = Bytes;
    type Output = File;

    fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        self.inner.write_all(&chunk)?;

        Ok(())
    }

    fn flush(mut self) -> Result<Self::Output> {
        self.inner.flush()?;

        let output = self.inner.into_inner()?;

        Ok(output)
    }
}
