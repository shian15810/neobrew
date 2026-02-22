use std::{
    io::BufRead,
    path::{Path, PathBuf},
};

use anyhow::Result;
use flate2::bufread::GzDecoder;
use tar::Archive;

use super::PipeOperator;

pub struct Pourer {
    path: PathBuf,
}

impl Pourer {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl PipeOperator for Pourer {
    type Output = PathBuf;

    fn from_reader(self, reader: impl BufRead) -> Result<Self::Output> {
        let gz_decoder = GzDecoder::new(reader);

        let mut archive = Archive::new(gz_decoder);

        archive.unpack(&self.path)?;

        let output = self.path;

        Ok(output)
    }
}
