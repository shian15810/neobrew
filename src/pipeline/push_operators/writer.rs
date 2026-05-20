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

use super::{super::AtomicFsHandler, PushOperator};
use crate::package::PreparedPackageCache;

pub(crate) struct Writer {
    buf_tmp_file: BufWriter<NamedTempFile>,

    cache: PreparedPackageCache,
}

impl Writer {
    pub(crate) async fn create(cache: PreparedPackageCache) -> Result<Self> {
        let handle = task::spawn_blocking(move || {
            fs::create_dir_all(&cache.file_location_parent)?;

            let tmp_file = NamedTempFile::new_in(&cache.file_location_parent)?;

            let this = Self {
                buf_tmp_file: BufWriter::new(tmp_file),

                cache,
            };

            anyhow::Ok(this)
        });
        let handle = AbortOnDropHandle::new(handle);

        let this = handle.await??;

        Ok(this)
    }
}

impl PushOperator for Writer {
    type Item = Bytes;
    type Output = WrittenTempFile;

    fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        self.buf_tmp_file.write_all(&chunk)?;

        Ok(())
    }

    fn flush(mut self) -> Result<Self::Output> {
        self.buf_tmp_file.flush()?;

        let written_temp_file = WrittenTempFile::try_from(self)?;

        Ok(written_temp_file)
    }
}

pub(crate) struct WrittenTempFile {
    tmp_file: NamedTempFile,

    cache: PreparedPackageCache,
}

impl TryFrom<Writer> for WrittenTempFile {
    type Error = anyhow::Error;

    fn try_from(writer: Writer) -> Result<Self, Self::Error> {
        let tmp_file = writer.buf_tmp_file.into_inner()?;

        let this = Self {
            tmp_file,

            cache: writer.cache,
        };

        Ok(this)
    }
}

impl AtomicFsHandler for WrittenTempFile {
    async fn cleanup(self) -> Result<()> {
        let handle = task::spawn_blocking(move || {
            self.tmp_file.close()?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }

    async fn persist(self) -> Result<()> {
        let handle = task::spawn_blocking(move || {
            self.tmp_file.persist(&self.cache.file_location)?;

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
