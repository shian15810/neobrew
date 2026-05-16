use std::{
    fs::Dir,
    io::BufRead,
    path::{Path, PathBuf},
};

use anyhow::Result;
use flate2::bufread::GzDecoder;
use tar::Archive;

use super::PullOperator;

pub(crate) struct Pourer {
    fetch_dest: PathBuf,
}

impl Pourer {
    pub(crate) fn new(fetch_dest: impl AsRef<Path>) -> Self {
        Self {
            fetch_dest: fetch_dest.as_ref().to_path_buf(),
        }
    }
}

impl PullOperator for Pourer {
    type Output = Dir;

    fn from_reader(self, reader: impl BufRead) -> Result<Self::Output> {
        let gz_decoder = GzDecoder::new(reader);

        let mut archive = Archive::new(gz_decoder);

        archive.unpack(&self.fetch_dest)?;

        let fetch_dest_dir = Dir::open(self.fetch_dest)?;

        Ok(fetch_dest_dir)
    }
}
