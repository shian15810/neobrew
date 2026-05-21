use std::{
    fs,
    io::{BufWriter, Write as _},
    os::unix::fs as unix_fs,
};

use anyhow::Result;
use bytes::Bytes;
use tempfile::NamedTempFile;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

use super::{super::handler, PushOperator};
use crate::package::prepared::PreparedPackageCache;

pub(crate) struct TempWriter {
    buf_file: BufWriter<NamedTempFile>,

    cache: PreparedPackageCache,
}

impl TempWriter {
    pub(crate) async fn create(cache: PreparedPackageCache) -> Result<Self> {
        let handle = task::spawn_blocking(move || {
            fs::create_dir_all(&cache.file_location_parent)?;

            let file = NamedTempFile::new_in(&cache.file_location_parent)?;

            let this = Self {
                buf_file: BufWriter::new(file),

                cache,
            };

            anyhow::Ok(this)
        });
        let handle = AbortOnDropHandle::new(handle);

        let this = handle.await??;

        Ok(this)
    }
}

impl PushOperator for TempWriter {
    type Item = Bytes;
    type Output = WrittenTempFile;

    fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        self.buf_file.write_all(&chunk)?;

        Ok(())
    }

    fn flush(mut self) -> Result<Self::Output> {
        self.buf_file.flush()?;

        let written_temp_file = WrittenTempFile::try_from(self)?;

        Ok(written_temp_file)
    }
}

pub(crate) struct WrittenTempFile {
    file: NamedTempFile,

    cache: PreparedPackageCache,
}

impl TryFrom<TempWriter> for WrittenTempFile {
    type Error = anyhow::Error;

    fn try_from(temp_writer: TempWriter) -> Result<Self, Self::Error> {
        let file = temp_writer.buf_file.into_inner()?;

        let this = Self {
            file,

            cache: temp_writer.cache,
        };

        Ok(this)
    }
}

impl handler::AtomicWriter for WrittenTempFile {
    async fn cleanup(self) -> Result<()> {
        let handle = task::spawn_blocking(move || {
            self.file.close()?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }

    async fn persist(self) -> Result<()> {
        let handle = task::spawn_blocking(move || {
            self.file.persist(&self.cache.file_location)?;

            unix_fs::symlink(
                self.cache.symlink_location_diff()?,
                self.cache.symlink_location_tmp(),
            )?;

            fs::rename(
                self.cache.symlink_location_tmp(),
                self.cache.symlink_location,
            )?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }
}
