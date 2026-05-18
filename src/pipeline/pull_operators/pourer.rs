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

use super::PullOperator;
use crate::package::PreparedPackageFetchDest;

pub(crate) struct Pourer {
    id: String,
    version: String,

    fetch_dest: PreparedPackageFetchDest,
}

impl Pourer {
    pub(crate) fn new(id: String, version: String, fetch_dest: PreparedPackageFetchDest) -> Self {
        Self {
            id,
            version,

            fetch_dest,
        }
    }
}

impl PullOperator for Pourer {
    type Output = ();

    fn from_reader(self, reader: impl BufRead) -> Result<Self::Output> {
        fs::create_dir_all(&self.fetch_dest.dir_location_grandparent)?;

        let tmp_dir = TempDir::new_in(self.fetch_dest.dir_location_grandparent)?;

        let gz_decoder = GzDecoder::new(reader);

        let mut archive = Archive::new(gz_decoder);

        archive.unpack(tmp_dir.path())?;

        let tmp_dir_entry = Self::fs_read_exactly_one_dir_entry(tmp_dir.path(), self.id)?;

        Self::fs_read_exactly_one_dir_entry(tmp_dir_entry.path(), self.version)?;

        if self.fetch_dest.dir_location_parent.exists() {
            fs::remove_dir_all(&self.fetch_dest.dir_location_parent)?;
        }

        fs::rename(tmp_dir_entry.path(), self.fetch_dest.dir_location_parent)?;

        Ok(())
    }
}

impl Pourer {
    fn fs_read_exactly_one_dir_entry(
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
