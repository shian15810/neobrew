use std::path::Path;

use tokio::{
    fs::File,
    io::{self, AsyncReadExt as _},
};

pub(crate) struct Elf;

impl Elf {
    const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
}

impl Elf {
    pub(crate) async fn has_magic(path: &Path) -> anyhow::Result<bool> {
        let mut file = File::open(path).await?;

        let mut peek_buf = [0_u8; 4];

        match file.read_exact(&mut peek_buf).await {
            Ok(_) => {},
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(false),
            Err(err) => return Err(err)?,
        }

        let has_magic = &peek_buf == Self::ELF_MAGIC;

        Ok(has_magic)
    }
}
