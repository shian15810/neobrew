use std::path::PathBuf;

use anyhow::Result;
use async_compression::tokio::bufread::GzipDecoder;
use async_zip::base::read::stream::ZipFileReader;
use tempfile::TempDir;
use tokio::{
    fs::{self, File},
    io::{self, AsyncRead, BufReader},
};
use tokio_tar::Archive;
use tokio_util::compat::FuturesAsyncReadCompatExt as _;

use super::{super::handler, PullOperator};
use crate::{
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    util::ArchiveFormat,
};

pub(crate) struct TempPourer {
    archive_format: ArchiveFormat,
    dir_path: PathBuf,
    symlink_paths: Vec<PathBuf>,
}

impl TempPourer {
    pub(crate) fn create(
        archive_format: ArchiveFormat,
        dir_path: PathBuf,
        symlink_paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            archive_format,
            dir_path,
            symlink_paths,
        }
    }
}

impl PullOperator for TempPourer {
    type Output = TempPourerOutput;

    async fn from_reader(self, reader: impl AsyncRead + Unpin + Send) -> Result<Self::Output> {
        fs::create_dir_all(&self.dir_path).await?;

        let dir = TempDir::new_in(&self.dir_path)?;

        let buf_reader = BufReader::new(reader);

        match self.archive_format {
            ArchiveFormat::TarGz => {
                let gz_decoder = GzipDecoder::new(buf_reader);

                let mut archive = Archive::new(gz_decoder);

                archive.unpack(dir.path()).await?;
            },
            ArchiveFormat::Zip => {
                let mut zip = ZipFileReader::with_tokio(buf_reader);

                while let Some(mut entry_reader) = zip.next_with_entry().await? {
                    let file_name = entry_reader.reader().entry().filename().as_str()?;

                    if file_name.ends_with('/') {
                        zip = entry_reader.skip().await?;
                    } else {
                        let dest_dir_path = dir.path();

                        let dest_path = dest_dir_path.join(file_name);

                        let dest_base_path = dest_path.base()?;

                        fs::create_dir_all(dest_base_path).await?;

                        let mut dest_file = File::create(dest_path).await?;

                        io::copy(&mut entry_reader.reader_mut().compat(), &mut dest_file).await?;

                        zip = entry_reader.done().await?;
                    }
                }
            },
        }

        let output = TempPourerOutput {
            dir,

            dir_path: self.dir_path,
            symlink_paths: self.symlink_paths,
        };

        Ok(output)
    }
}

pub(crate) struct TempPourerOutput {
    dir: TempDir,

    dir_path: PathBuf,
    symlink_paths: Vec<PathBuf>,
}

impl handler::AtomicWriter for TempPourerOutput {
    async fn cleanup(self) -> Result<()> {
        self.dir.close()?;

        Ok(())
    }

    async fn persist(self) -> Result<()> {
        let src_dir_path = self.dir.path();

        let dest_dir_path = self.dir_path;

        let mut src_dir_entries = fs::read_dir(src_dir_path).await?;

        while let Some(src_dir_entry) = src_dir_entries.next_entry().await? {
            let src_path = src_dir_entry.path();

            let dest_path = dest_dir_path.join(src_dir_entry.file_name());

            if !src_path.is_dir_exists_nofollow().await? {
                continue;
            }

            if dest_path.is_dir_exists_nofollow().await? {
                fs::remove_dir_all(&dest_path).await?;
            }

            fs::rename(src_path, dest_path).await?;
        }

        self.dir.close()?;

        for symlink_path in self.symlink_paths {
            dest_dir_path
                .create_relative_symlink_atomically_at(symlink_path)
                .await?;
        }

        Ok(())
    }
}
