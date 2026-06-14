use std::{io::SeekFrom, path::Path};

use anyhow::{Context as _, anyhow};
use async_compression::tokio::bufread::{
    BzDecoder,
    GzipDecoder,
    LzmaDecoder,
    XzDecoder,
    ZstdDecoder,
};
use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncBufRead, AsyncRead, AsyncReadExt as _, AsyncSeekExt as _, BufReader},
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
            "pkg" => Self::Pkg,
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
    const PEEK_SIZE: usize = 262;

    pub(crate) async fn peek(
        buf_reader: &mut (impl AsyncBufRead + Unpin),
    ) -> anyhow::Result<(Self, [u8; Self::PEEK_SIZE])> {
        let mut peek_buf = [0_u8; Self::PEEK_SIZE];

        buf_reader.read_exact(&mut peek_buf).await?;

        let kind = infer::get(&peek_buf).context("Failed to detect archive format")?;

        let archive_format = match kind.extension() {
            "tar" => Self::Tar,
            "bz2" => {
                let buf_reader = BufReader::new(peek_buf.as_ref());

                let bz_decoder = BzDecoder::new(buf_reader);

                if !Self::peek_tar(bz_decoder).await {
                    let err = anyhow!("Unsupported archive format detected within bzip2");

                    return Err(err);
                }

                Self::TarBzip2
            },
            "gz" => {
                let buf_reader = BufReader::new(peek_buf.as_ref());

                let gzip_decoder = GzipDecoder::new(buf_reader);

                if !Self::peek_tar(gzip_decoder).await {
                    let err = anyhow!("Unsupported archive format detected within gzip");

                    return Err(err);
                }

                Self::TarGzip
            },
            "lzma" => {
                let buf_reader = BufReader::new(peek_buf.as_ref());

                let lzma_decoder = LzmaDecoder::new(buf_reader);

                if !Self::peek_tar(lzma_decoder).await {
                    let err = anyhow!("Unsupported archive format detected within lzma");

                    return Err(err);
                }

                Self::TarLzma
            },
            "xz" => {
                let buf_reader = BufReader::new(peek_buf.as_ref());

                let xz_decoder = XzDecoder::new(buf_reader);

                if !Self::peek_tar(xz_decoder).await {
                    let err = anyhow!("Unsupported archive format detected within xz");

                    return Err(err);
                }

                Self::TarXz
            },
            "zst" => {
                let buf_reader = BufReader::new(peek_buf.as_ref());

                let zstd_decoder = ZstdDecoder::new(buf_reader);

                if !Self::peek_tar(zstd_decoder).await {
                    let err = anyhow!("Unsupported archive format detected within zstd");

                    return Err(err);
                }

                Self::TarZstd
            },
            "zip" => Self::Zip,
            extension => {
                let err = anyhow!(r#"Unsupported archive format detected: "{extension}""#);

                return Err(err);
            },
        };

        Ok((archive_format, peek_buf))
    }

    async fn peek_tar(mut decoder: impl AsyncRead + Unpin) -> bool {
        let mut peek_buf = [0_u8; Self::PEEK_SIZE];

        if decoder.read_exact(&mut peek_buf).await.is_err() {
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

#[derive(Debug, Error)]
pub(crate) enum ArchiveFormatError {
    #[error("Unsupported archive format detected")]
    Unsupported,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
