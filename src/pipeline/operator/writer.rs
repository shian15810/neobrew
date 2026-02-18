use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::Result;
use bytes::Bytes;

use super::Operator;

pub struct Writer {
    inner: BufWriter<File>,
}

impl Writer {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            inner: BufWriter::new(File::create(path)?),
        })
    }
}

impl Operator<Bytes, File> for Writer {
    fn feed(&mut self, chunk: Bytes) -> Result<()> {
        self.inner.write_all(&chunk)?;

        Ok(())
    }

    fn flush(mut self) -> Result<File> {
        self.inner.flush()?;

        let output = self.inner.into_inner()?;

        Ok(output)
    }
}
