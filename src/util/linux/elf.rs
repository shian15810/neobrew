pub(crate) struct Elf;

impl Elf {
    const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
}

impl Elf {
    pub(crate) fn has_magic(bytes: &[u8]) -> bool {
        let &[b0, b1, b2, b3, ..] = bytes else {
            return false;
        };

        let header_bytes = &[b0, b1, b2, b3];

        header_bytes == Self::ELF_MAGIC
    }
}
