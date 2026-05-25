#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
pub(crate) use self::linux::Relocation;
#[cfg(target_os = "macos")]
pub(crate) use self::macos::Relocation;
