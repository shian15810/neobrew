pub(crate) struct MachO;

impl MachO {
    const FAT_MAGIC: u32 = 0xcafe_babe;
    const FAT_CIGAM: u32 = 0xbeba_feca;
    const FAT_MAGIC_64: u32 = 0xcafe_babf;
    const FAT_CIGAM_64: u32 = 0xbfba_feca;

    const BE_MAGIC_NUMBERS: &[u32] = &[
        Self::FAT_MAGIC,
        Self::FAT_MAGIC_64,
        Self::MH_MAGIC,
        Self::MH_MAGIC_64,
    ];

    const MH_MAGIC: u32 = 0xfeed_face;
    const MH_CIGAM: u32 = 0xcefa_edfe;
    const MH_MAGIC_64: u32 = 0xfeed_facf;
    const MH_CIGAM_64: u32 = 0xcffa_edfe;

    const LE_MAGIC_NUMBERS: &[u32] = &[
        Self::FAT_CIGAM,
        Self::FAT_CIGAM_64,
        Self::MH_CIGAM,
        Self::MH_CIGAM_64,
    ];
}

impl MachO {
    pub(crate) fn has_magic_number(bytes: &[u8]) -> bool {
        let &[b0, b1, b2, b3, ..] = bytes else {
            return false;
        };

        let header_bytes = &[b0, b1, b2, b3];

        let be_magic_number = u32::from_be_bytes(*header_bytes);

        if Self::BE_MAGIC_NUMBERS.contains(&be_magic_number) {
            return true;
        }

        let le_magic_number = u32::from_le_bytes(*header_bytes);

        if Self::LE_MAGIC_NUMBERS.contains(&le_magic_number) {
            return true;
        }

        false
    }
}
