#[cfg(target_os = "linux")]
pub(crate) use super::linux::Relocation;
#[cfg(target_os = "macos")]
pub(crate) use super::macos::Relocation;
