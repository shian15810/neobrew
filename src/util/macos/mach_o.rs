pub(crate) struct MachO;

impl MachO {
    const FAT_MAGIC: u32 = 0xcafe_babe;
    const FAT_CIGAM: u32 = 0xbeba_feca;
    const FAT_MAGIC_64: u32 = 0xcafe_babf;
    const FAT_CIGAM_64: u32 = 0xbfba_feca;

    const BE_MAGICS: &[u32] = &[
        Self::FAT_MAGIC,
        Self::FAT_MAGIC_64,
        Self::MH_MAGIC,
        Self::MH_MAGIC_64,
    ];

    const MH_MAGIC: u32 = 0xfeed_face;
    const MH_CIGAM: u32 = 0xcefa_edfe;
    const MH_MAGIC_64: u32 = 0xfeed_facf;
    const MH_CIGAM_64: u32 = 0xcffa_edfe;

    const LE_MAGICS: &[u32] = &[
        Self::FAT_CIGAM,
        Self::FAT_CIGAM_64,
        Self::MH_CIGAM,
        Self::MH_CIGAM_64,
    ];
}

impl MachO {
    pub(crate) fn has_magic(bytes: &[u8]) -> bool {
        let &[b0, b1, b2, b3, ..] = bytes else {
            return false;
        };

        let header_bytes = &[b0, b1, b2, b3];

        let peek_magic = u32::from_be_bytes(*header_bytes);

        if Self::BE_MAGICS.contains(&peek_magic) {
            return true;
        }

        let peek_magic = u32::from_le_bytes(*header_bytes);

        if Self::LE_MAGICS.contains(&peek_magic) {
            return true;
        }

        false
    }
}
