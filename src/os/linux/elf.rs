use anyhow::{Result, anyhow};

pub(super) struct Elf;

impl Elf {
    const MAGIC_NUMBER: &[u8; 4] = b"\x7fELF";
}

impl Elf {
    pub(super) fn has_magic_number(bytes: &[u8]) -> Result<bool> {
        let &[b0, b1, b2, b3, ..] = bytes else {
            let err = anyhow!("Not enough header bytes");

            return Err(err);
        };

        let header_bytes = &[b0, b1, b2, b3];

        let has_magic_number = header_bytes == Self::MAGIC_NUMBER;

        Ok(has_magic_number)
    }
}
