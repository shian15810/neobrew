use std::{
    fs,
    io::{BufWriter, Write as _},
    os::unix::fs as unix_fs,
};

use anyhow::Result;
use bytes::Bytes;
use tempfile::NamedTempFile;

use super::PushOperator;
use crate::package::PreparedPackageFetchCache;

pub(crate) struct Writer {
    buf_file: BufWriter<NamedTempFile>,

    fetch_cache: PreparedPackageFetchCache,
}

impl Writer {
    pub(crate) fn new(fetch_cache: PreparedPackageFetchCache) -> Result<Self> {
        fs::create_dir_all(&fetch_cache.file_location_parent)?;

        let file = NamedTempFile::new_in(&fetch_cache.file_location_parent)?;

        let this = Self {
            buf_file: BufWriter::new(file),

            fetch_cache,
        };

        Ok(this)
    }
}

impl PushOperator for Writer {
    type Item = Bytes;
    type Output = ();

    fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        self.buf_file.write_all(&chunk)?;

        Ok(())
    }

    fn flush(mut self) -> Result<Self::Output> {
        self.buf_file.flush()?;

        let file = self.buf_file.into_inner()?;

        file.persist(self.fetch_cache.file_location)?;

        unix_fs::symlink(
            self.fetch_cache.symlink_location_diff,
            &self.fetch_cache.symlink_location_tmp,
        )?;

        fs::rename(
            self.fetch_cache.symlink_location_tmp,
            self.fetch_cache.symlink_location,
        )?;

        Ok(())
    }
}
