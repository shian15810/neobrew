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
use crate::package::PreparedPackageDest;

pub(crate) struct Pourer {
    dest: PreparedPackageDest,
}

impl From<PreparedPackageDest> for Pourer {
    fn from(dest: PreparedPackageDest) -> Self {
        Self {
            dest,
        }
    }
}

impl PullOperator for Pourer {
    type Output = PouredTempDest;

    fn from_reader(self, reader: impl BufRead) -> Result<Self::Output> {
        fs::create_dir_all(&self.dest.dir_location_grandparent)?;

        let tmp_dir = TempDir::new_in(&self.dest.dir_location_grandparent)?;

        let gz_decoder = GzDecoder::new(reader);

        let mut archive = Archive::new(gz_decoder);

        archive.unpack(tmp_dir.path())?;

        let poured_temp_dest = PouredTempDest::from((tmp_dir, self));

        Ok(poured_temp_dest)
    }
}

pub(crate) struct PouredTempDest {
    tmp_dir: TempDir,

    dest: PreparedPackageDest,
}

impl From<(TempDir, Pourer)> for PouredTempDest {
    fn from((tmp_dir, pourer): (TempDir, Pourer)) -> Self {
        Self {
            tmp_dir,

            dest: pourer.dest,
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
            if self.dest.dir_location_parent.is_dir() {
                fs::remove_dir_all(&self.dest.dir_location_parent)?;
            }

            fs::rename(self.tmp_dir_entry()?.path(), self.dest.dir_location_parent)?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }
}

impl PouredTempDest {
    fn tmp_dir_entry(&self) -> Result<DirEntry> {
        let entry = Self::exactly_one_tmp_dir_entry(self.tmp_dir.path(), &self.dest.id)?;

        Self::exactly_one_tmp_dir_entry(entry.path(), &self.dest.version)?;

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
