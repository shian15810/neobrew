#[cfg(target_os = "linux")]
mod elf;

#[cfg(target_os = "linux")]
pub(crate) use self::elf::Elf;
