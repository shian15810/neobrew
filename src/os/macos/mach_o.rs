use anyhow::{Result, anyhow};

struct MachO {
    magic: u32,
}

impl MachO {
    const FAT_MAGIC: u32 = 0xcafe_babe;
    const FAT_CIGAM: u32 = 0xbeba_feca;
    const FAT_MAGIC_64: u32 = 0xcafe_babf;
    const FAT_CIGAM_64: u32 = 0xbfba_feca;
    const MH_MAGIC: u32 = 0xfeed_face;
    const MH_CIGAM: u32 = 0xcefa_edfe;
    const MH_MAGIC_64: u32 = 0xfeed_facf;
    const MH_CIGAM_64: u32 = 0xcffa_edfe;

    const MAGICS: &[u32] = &[
        Self::FAT_MAGIC,
        Self::FAT_CIGAM,
        Self::FAT_MAGIC_64,
        Self::FAT_CIGAM_64,
        Self::MH_MAGIC,
        Self::MH_CIGAM,
        Self::MH_MAGIC_64,
        Self::MH_CIGAM_64,
    ];
}

impl MachO {
    fn detect(bytes: &[u8]) -> Result<Self> {
        let Some(bytes) = bytes.get(0..4) else {
            let err = anyhow!("Not enough bytes");

            return Err(err);
        };

        let bytes: &[u8; 4] = bytes.try_into()?;

        let bytes = match Self::try_from(bytes) {
            Ok(bytes) => bytes,
            Err(Some(err)) => return Err(err),
            Err(None) => {
                let err = anyhow!("No Mach-O magic number detected");

                return Err(err);
            },
        };

        Ok(bytes)
    }
}

impl TryFrom<&[u8; 4]> for MachO {
    type Error = Option<anyhow::Error>;

    fn try_from(bytes: &[u8; 4]) -> Result<Self, Self::Error> {
        if let Some(be_magic) = match u32::from_be_bytes(*bytes) {
            Self::FAT_MAGIC => Some(Self::FAT_MAGIC),
            Self::FAT_MAGIC_64 => Some(Self::FAT_MAGIC_64),
            Self::MH_MAGIC => Some(Self::MH_MAGIC),
            Self::MH_MAGIC_64 => Some(Self::MH_MAGIC_64),
            _ => None,
        } {
            let this = Self {
                magic: be_magic,
            };

            return Ok(this);
        }

        if let Some(le_magic) = match u32::from_le_bytes(*bytes) {
            Self::FAT_CIGAM => Some(Self::FAT_CIGAM),
            Self::FAT_CIGAM_64 => Some(Self::FAT_CIGAM_64),
            Self::MH_CIGAM => Some(Self::MH_CIGAM),
            Self::MH_CIGAM_64 => Some(Self::MH_CIGAM_64),
            _ => None,
        } {
            let this = Self {
                magic: le_magic,
            };

            return Ok(this);
        }

        Err(None)
    }
}
