use std::{io::ErrorKind, path::Path};

use tokio::{fs::File, io::AsyncReadExt as _};

pub(crate) struct MachO;

impl MachO {
    const FAT_MAGIC: u32 = 0xcafe_babe;
    const FAT_CIGAM: u32 = 0xbeba_feca;
    const FAT_MAGIC_64: u32 = 0xcafe_babf;
    const FAT_CIGAM_64: u32 = 0xbfba_feca;

    const MH_MAGIC: u32 = 0xfeed_face;
    const MH_CIGAM: u32 = 0xcefa_edfe;
    const MH_MAGIC_64: u32 = 0xfeed_facf;
    const MH_CIGAM_64: u32 = 0xcffa_edfe;

    const BE_MAGICS: &[u32] = &[
        Self::FAT_MAGIC,
        Self::FAT_MAGIC_64,
        Self::MH_MAGIC,
        Self::MH_MAGIC_64,
    ];

    const LE_MAGICS: &[u32] = &[
        Self::FAT_CIGAM,
        Self::FAT_CIGAM_64,
        Self::MH_CIGAM,
        Self::MH_CIGAM_64,
    ];
}

impl MachO {
    pub(crate) async fn has_magic(path: &Path) -> anyhow::Result<bool> {
        let mut file = File::open(path).await?;

        let mut peek_buf = [0_u8; 4];

        match file.read_exact(&mut peek_buf).await {
            Ok(_) => {},
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => return Ok(false),
            Err(err) => return Err(err)?,
        }

        let peek_magic = u32::from_be_bytes(peek_buf);

        if Self::BE_MAGICS.contains(&peek_magic) {
            return Ok(true);
        }

        let peek_magic = u32::from_le_bytes(peek_buf);

        if Self::LE_MAGICS.contains(&peek_magic) {
            return Ok(true);
        }

        Ok(false)
    }
}
