mod linker;
#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
pub(crate) mod relocation;

pub(crate) use self::{linker::Linker, relocation::Relocation};
