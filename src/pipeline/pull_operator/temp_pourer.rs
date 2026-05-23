use std::{fs, io::BufRead, path::PathBuf};

use anyhow::Result;
use flate2::bufread::GzDecoder;
use tar::Archive;
use tempfile::TempDir;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

use super::{super::handler, PullOperator};

pub(crate) struct TempPourerInput {
    pub(crate) dir_path: PathBuf,
}

impl TempPourerInput {
    pub(crate) fn new(dir_path: PathBuf) -> Self {
        Self {
            dir_path,
        }
    }
}

pub(crate) struct TempPourer {
    input: TempPourerInput,
}

impl TempPourer {
    pub(crate) fn create(input: TempPourerInput) -> Self {
        Self {
            input,
        }
    }
}

impl PullOperator for TempPourer {
    type Output = TempPourerOutput;

    fn from_reader(self, reader: impl BufRead) -> Result<Self::Output> {
        fs::create_dir_all(&self.input.dir_path)?;

        let dir = TempDir::new_in(&self.input.dir_path)?;

        let gz_decoder = GzDecoder::new(reader);

        let mut archive = Archive::new(gz_decoder);

        archive.unpack(dir.path())?;

        let temp_pourer_output = TempPourerOutput::from((dir, self));

        Ok(temp_pourer_output)
    }
}

pub(crate) struct TempPourerOutput {
    dir: TempDir,

    input: TempPourerInput,
}

impl From<(TempDir, TempPourer)> for TempPourerOutput {
    fn from((dir, temp_pourer): (TempDir, TempPourer)) -> Self {
        Self {
            dir,

            input: temp_pourer.input,
        }
    }
}

impl handler::AtomicWriter for TempPourerOutput {
    async fn cleanup(self) -> Result<()> {
        let handle = task::spawn_blocking(move || {
            self.dir.close()?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }

    async fn persist(self) -> Result<()> {
        let src_dir_path = self.dir;

        let dest_dir_path = self.input.dir_path;

        let handle = task::spawn_blocking(move || {
            for entry in fs::read_dir(&src_dir_path)? {
                let entry = entry?;

                let src_path = entry.path();

                let dest_path = dest_dir_path.join(entry.file_name());

                if !src_path.is_dir() {
                    continue;
                }

                if dest_path.is_dir() {
                    fs::remove_dir_all(&dest_path)?;
                }

                fs::rename(src_path, dest_path)?;
            }

            src_dir_path.close()?;

            anyhow::Ok(())
        });
        let handle = AbortOnDropHandle::new(handle);

        handle.await??;

        Ok(())
    }
}
