use std::{
    ffi::OsStr,
    fs::{self, DirEntry},
    io::BufRead,
    path::Path,
};

use anyhow::{Result, anyhow};
use flate2::bufread::GzDecoder;
use itertools::Itertools as _;
use tar::Archive;
use tempfile::TempDir;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

use super::{super::AtomicFsHandler, PullOperator};
use crate::package::PreparedPackageFetchDest;

pub(crate) struct Pourer {
    fetch_dest: PreparedPackageFetchDest,
}

impl From<PreparedPackageFetchDest> for Pourer {
    fn from(fetch_dest: PreparedPackageFetchDest) -> Self {
        Self {
            fetch_dest,
        }
    }
}

impl PullOperator for Pourer {
    type Output = PouredTempDest;

    fn from_reader(self, reader: impl BufRead) -> Result<Self::Output> {
        fs::create_dir_all(&self.fetch_dest.dir_location_grandparent)?;

        let tmp_dir = TempDir::new_in(&self.fetch_dest.dir_location_grandparent)?;

        let gz_decoder = GzDecoder::new(reader);

        let mut archive = Archive::new(gz_decoder);

        archive.unpack(tmp_dir.path())?;

        let poured_temp_dest = PouredTempDest::from((tmp_dir, self));

        Ok(poured_temp_dest)
    }
}

pub(crate) struct PouredTempDest {
    tmp_dir: TempDir,

    fetch_dest: PreparedPackageFetchDest,
}

impl From<(TempDir, Pourer)> for PouredTempDest {
    fn from((tmp_dir, pourer): (TempDir, Pourer)) -> Self {
        Self {
            tmp_dir,

            fetch_dest: pourer.fetch_dest,
        }
    }
}

impl AtomicFsHandler for PouredTempDest {
    async fn cleanup(self) -> Result<()> {
        let handle = task::spawn_blocking(move || {
            self.tmp_dir.close()?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }

    async fn persist(self) -> Result<()> {
        let handle = task::spawn_blocking(move || {
            if self.fetch_dest.dir_location_parent.exists() {
                fs::remove_dir_all(&self.fetch_dest.dir_location_parent)?;
            }

            fs::rename(
                self.tmp_dir_entry()?.path(),
                self.fetch_dest.dir_location_parent,
            )?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }
}

impl PouredTempDest {
    fn tmp_dir_entry(&self) -> Result<DirEntry> {
        let entry = Self::exactly_one_tmp_dir_entry(self.tmp_dir.path(), &self.fetch_dest.id)?;

        Self::exactly_one_tmp_dir_entry(entry.path(), &self.fetch_dest.version)?;

        Ok(entry)
    }

    fn exactly_one_tmp_dir_entry(
        path: impl AsRef<Path>,
        expected_name: impl AsRef<OsStr>,
    ) -> Result<DirEntry> {
        let expected_name = expected_name.as_ref();

        let entry = fs::read_dir(path)?.exactly_one()??;

        let actual_name = entry.file_name();

        if entry.path().is_dir() && actual_name == expected_name {
            return Ok(entry);
        }

        let actual_name = actual_name.display();

        let expected_name = expected_name.display();

        let err =
            anyhow!(r#"Expected a directory named "{expected_name}" but found "{actual_name}""#);

        Err(err)
    }
}
