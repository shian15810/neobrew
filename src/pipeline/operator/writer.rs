use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use bytes::Bytes;

use super::Operator;

pub struct Writer {
    inner: Option<BufWriter<File>>,

    path: PathBuf,
}

impl Writer {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            inner: None,
        }
    }

    fn get_or_init(&mut self) -> Result<&mut BufWriter<File>> {
        if self.inner.is_none() {
            let file = File::create(&self.path)?;
            let inner = BufWriter::new(file);

            self.inner = Some(inner);
        }

        Ok(self.inner.as_mut().unwrap())
    }
}

impl Operator for Writer {
    fn send(&mut self, chunk: Bytes) -> Result<()> {
        let this = self.get_or_init()?;

        this.write_all(&chunk)?;

        Ok(())
    }

    fn apply(mut self) -> Result<()> {
        let this = self.get_or_init()?;

        this.flush()?;

        Ok(())
    }
}
