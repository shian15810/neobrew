use std::io::BufRead;

use anyhow::Result;

use super::PipeOperator;

pub struct Pourer;

impl Pourer {
    pub fn new() -> Self {
        Self
    }
}

impl PipeOperator<()> for Pourer {
    fn from_reader(self, _reader: impl BufRead) -> Result<()> {
        let output = ();

        Ok(output)
    }
}
