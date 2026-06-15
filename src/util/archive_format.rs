use std::path::Path;

use anyhow::{Context as _, anyhow};
use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncBufRead, AsyncReadExt as _, AsyncSeekExt as _, SeekFrom},
};

use crate::ext::std::path::PathExt as _;

#[derive(Clone, Copy)]
pub(crate) enum ArchiveFormat {
    Dmg,
    Pkg,
    Tar,
    TarBzip2,
    TarGzip,
    TarLzma,
    TarXz,
    TarZstd,
    Zip,
}

impl TryFrom<&str> for ArchiveFormat {
    type Error = ArchiveFormatError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        let path = Path::new(name);

        Self::try_from(path)
    }
}

impl TryFrom<&Path> for ArchiveFormat {
    type Error = ArchiveFormatError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let Some(compound_extension) = path.compound_extension() else {
            return Err(ArchiveFormatError::Unsupported);
        };

        let compound_extension = compound_extension.to_string_lossy();
        let compound_extension = compound_extension.as_ref();

        let archive_format = match compound_extension {
            "dmg" => Self::Dmg,
            "pkg" | "mpkg" => Self::Pkg,
            "tar" => Self::Tar,
            "tar.bz2" | "tbz2" | "tbz" => Self::TarBzip2,
            "tar.gz" | "tgz" | "crate" => Self::TarGzip,
            "tar.lzma" | "tlzma" => Self::TarLzma,
            "tar.xz" | "txz" => Self::TarXz,
            "tar.zst" | "tzst" => Self::TarZstd,
            "zip" => Self::Zip,
            _ => return Err(ArchiveFormatError::Unsupported),
        };

        Ok(archive_format)
    }
}

impl ArchiveFormat {
    const PEEK_SIZE: u64 = 262;

    pub(crate) async fn peek(
        buf_reader: &mut (impl AsyncBufRead + Unpin),
    ) -> anyhow::Result<(Self, Vec<u8>)> {
        let mut peek_buf = Vec::new();

        buf_reader
            .take(Self::PEEK_SIZE)
            .read_to_end(&mut peek_buf)
            .await?;

        let kind = infer::get(&peek_buf).context("Failed to peek archive format")?;

        let archive_format = match kind.extension() {
            "tar" => Self::Tar,
            "bz2" => Self::TarBzip2,
            "gz" => Self::TarGzip,
            "lzma" => Self::TarLzma,
            "xz" => Self::TarXz,
            "zst" => Self::TarZstd,
            "zip" => Self::Zip,
            extension => {
                let err = anyhow!(r#"Unsupported archive format detected: "{extension}""#);

                return Err(err);
            },
        };

        Ok((archive_format, peek_buf))
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

        let mut peek_buf = [0_u8; 4];

        file.read_exact(&mut peek_buf).await?;

        let is_dmg = &peek_buf == KOLY_MAGIC;

        Ok(is_dmg)
    }
}

#[derive(Debug, Error)]
pub(crate) enum ArchiveFormatError {
    #[error("Unsupported archive format detected")]
    Unsupported,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
