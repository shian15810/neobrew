mod archive_format;
#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;

pub(crate) use self::archive_format::ArchiveFormat;
