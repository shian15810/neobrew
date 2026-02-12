use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use bytes::Bytes;

use super::Operator;

pub struct FileWriter {
    path: PathBuf,
    sink: Option<BufWriter<File>>,
}

impl FileWriter {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            sink: None,
        }
    }

    fn get_or_init_sink(&mut self) -> Result<&mut BufWriter<File>> {
        if self.sink.is_none() {
            let file = File::create(&self.path)?;
            let sink = BufWriter::new(file);

            self.sink = Some(sink);
        }

        Ok(self.sink.as_mut().unwrap())
    }
}

impl Operator for FileWriter {
    fn send(&mut self, chunk: Bytes) -> Result<()> {
        let sink = self.get_or_init_sink()?;

        sink.write_all(&chunk)?;

        Ok(())
    }

    fn apply(mut self) -> Result<()> {
        let sink = self.get_or_init_sink()?;

        sink.flush()?;

        Ok(())
    }
}
