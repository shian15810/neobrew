use std::path::Path;

use anyhow::{Context as _, Result};

pub(crate) trait PathExt {
    fn base(&self) -> Result<&Self>;
}

impl PathExt for Path {
    fn base(&self) -> Result<&Self> {
        let base_path = self.parent().context("No parent directory found")?;

        Ok(base_path)
    }
}
