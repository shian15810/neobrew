use std::{
    fs::{self, File},
    io::{BufWriter, Write as _},
    os::unix::fs as unix_fs,
};

use anyhow::Result;
use bytes::Bytes;

use super::PushOperator;
use crate::package::{PreparedPackageFetchCache, PreparedPackageFetchCacheFiles};

pub(crate) struct Writer {
    inner: BufWriter<File>,

    fetch_cache: PreparedPackageFetchCache,
}

impl Writer {
    pub(crate) fn new(fetch_cache: PreparedPackageFetchCache) -> Result<Self> {
        fs::create_dir_all(&fetch_cache.file_location_parent)?;

        let file = File::create(&fetch_cache.file_location)?;

        let this = Self {
            inner: BufWriter::new(file),

            fetch_cache,
        };

        Ok(this)
    }
}

impl PushOperator for Writer {
    type Item = Bytes;
    type Output = PreparedPackageFetchCacheFiles;

    fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        self.inner.write_all(&chunk)?;

        Ok(())
    }

    fn flush(mut self) -> Result<Self::Output> {
        self.inner.flush()?;

        fs::create_dir_all(self.fetch_cache.symlink_location_parent)?;

        unix_fs::symlink(
            self.fetch_cache.file_location,
            &self.fetch_cache.symlink_location,
        )?;

        let symlink_file = File::open(self.fetch_cache.symlink_location)?;

        let file_file = self.inner.into_inner()?;

        let fetch_cache_files = PreparedPackageFetchCacheFiles {
            file_file,

            symlink_file,
        };

        Ok(fetch_cache_files)
    }
}
