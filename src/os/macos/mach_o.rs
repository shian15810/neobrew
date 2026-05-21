use anyhow::{Result, anyhow};

pub(super) struct MachO {
    magic_number: u32,
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

    const BE_MAGIC_NUMBERS: &[u32] = &[
        Self::FAT_MAGIC,
        Self::FAT_MAGIC_64,
        Self::MH_MAGIC,
        Self::MH_MAGIC_64,
    ];

    const LE_MAGIC_NUMBERS: &[u32] = &[
        Self::FAT_CIGAM,
        Self::FAT_CIGAM_64,
        Self::MH_CIGAM,
        Self::MH_CIGAM_64,
    ];
}

impl TryFrom<&[u8; 4]> for MachO {
    type Error = Option<anyhow::Error>;

    fn try_from(bytes: &[u8; 4]) -> Result<Self, Self::Error> {
        let be_magic_number = u32::from_be_bytes(*bytes);

        if Self::BE_MAGIC_NUMBERS.contains(&be_magic_number) {
            let this = Self {
                magic_number: be_magic_number,
            };

            return Ok(this);
        }

        let le_magic_number = u32::from_le_bytes(*bytes);

        if Self::LE_MAGIC_NUMBERS.contains(&le_magic_number) {
            let this = Self {
                magic_number: le_magic_number,
            };

            return Ok(this);
        }

        Err(None)
    }
}

impl MachO {
    pub(super) fn detect_magic_number(bytes: &[u8]) -> Result<Option<u32>> {
        let Some(bytes) = bytes.get(0..4) else {
            let err = anyhow!("Not enough magic bytes");

            return Err(err);
        };

        let bytes: &[u8; 4] = bytes.try_into()?;

        let this = match Self::try_from(bytes) {
            Ok(bytes) => bytes,
            Err(Some(err)) => return Err(err),
            Err(None) => return Ok(None),
        };

        let magic_number = this.magic_number;

        Ok(Some(magic_number))
    }
}
