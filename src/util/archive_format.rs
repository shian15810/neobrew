use std::{io::SeekFrom, path::Path};

use anyhow::{Context as _, anyhow};
use async_compression::tokio::bufread::GzipDecoder;
use tokio::{
    fs::File,
    io::{AsyncReadExt as _, AsyncSeekExt as _, BufReader},
};

use crate::ext::std::path::PathExt as _;

#[derive(Clone, Copy)]
pub(crate) enum ArchiveFormat {
    Dmg,
    TarGz,
    Zip,
}

impl TryFrom<&Path> for ArchiveFormat {
    type Error = Option<anyhow::Error>;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let Some(compound_extension) = path.compound_extension() else {
            return Err(None);
        };

        let compound_extension = compound_extension.to_string_lossy();
        let compound_extension = compound_extension.as_ref();

        let archive_format = match compound_extension {
            "dmg" => Self::Dmg,
            "tar.gz" | "tgz" => Self::TarGz,
            "zip" => Self::Zip,
            _ => return Err(None),
        };

        Ok(archive_format)
    }
}

impl ArchiveFormat {
    pub(crate) const PEEK_SIZE: usize = 262;

    pub(crate) async fn detect(bytes: &[u8]) -> anyhow::Result<Self> {
        let kind = infer::get(bytes).context("Failed to detect archive format from magic bytes")?;

        let archive_format = match kind.extension() {
            "gz" => {
                if !Self::detect_tar_gz(bytes).await {
                    let err = anyhow!("Gzip stream has no tar archive within");

                    return Err(err);
                }

                Self::TarGz
            },
            "zip" => Self::Zip,
            extension => {
                let err = anyhow!(r#"Unsupported archive format detected: "{extension}""#);

                return Err(err);
            },
        };

        Ok(archive_format)
    }

    async fn detect_tar_gz(bytes: &[u8]) -> bool {
        let mut peek_buf = [0_u8; Self::PEEK_SIZE];

        let buf_reader = BufReader::new(bytes);

        let gz_decoder = GzipDecoder::new(buf_reader);

        let mut gz_buf_reader = BufReader::new(gz_decoder);

        if gz_buf_reader.read_exact(&mut peek_buf).await.is_err() {
            return false;
        }

        infer::archive::is_tar(&peek_buf)
    }

    pub(crate) async fn is_dmg(file_path: &Path) -> anyhow::Result<bool> {
        const KOLY_MAGIC: &[u8; 4] = b"koly";
        const KOLY_OFFSET: i64 = -512;
        const KOLY_SIZE: u64 = KOLY_OFFSET.unsigned_abs();

        let mut file = File::open(file_path).await?;

        let metadata = file.metadata().await?;

        if metadata.len() < KOLY_SIZE {
            return Ok(false);
        }

        file.seek(SeekFrom::End(KOLY_OFFSET)).await?;

        let mut peek_magic = [0_u8; 4];

        file.read_exact(&mut peek_magic).await?;

        let is_dmg = &peek_magic == KOLY_MAGIC;

        Ok(is_dmg)
    }
}
