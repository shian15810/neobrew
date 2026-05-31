use std::path::Path;

use anyhow::{Context as _, anyhow};
use async_compression::tokio::bufread::GzipDecoder;
use tokio::io::{AsyncReadExt as _, BufReader};

use crate::ext::std::path::PathExt as _;

#[derive(Clone)]
pub(crate) enum ArchiveFormat {
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
            "tar.gz" | "tgz" => Self::TarGz,
            "zip" => Self::Zip,
            _ => return Err(None),
        };

        Ok(archive_format)
    }
}

impl ArchiveFormat {
    pub(crate) const PEEK_BUF_SIZE: usize = 262;

    pub(crate) async fn detect(bytes: &[u8]) -> anyhow::Result<Self> {
        let kind = infer::get(bytes).context("Failed to detect archive format from magic bytes")?;

        let archive_format = match kind.extension() {
            "gz" => {
                if !Self::is_tar_gz(bytes).await {
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

    async fn is_tar_gz(bytes: &[u8]) -> bool {
        let mut peek_buf = [0_u8; Self::PEEK_BUF_SIZE];

        let buf_reader = BufReader::new(bytes);

        let gz_decoder = GzipDecoder::new(buf_reader);

        let mut gz_buf_reader = BufReader::new(gz_decoder);

        if gz_buf_reader.read_exact(&mut peek_buf).await.is_err() {
            return false;
        }

        infer::archive::is_tar(&peek_buf)
    }
}
