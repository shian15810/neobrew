mod codename;
#[cfg(target_os = "macos")]
mod codesign;
#[cfg(target_os = "macos")]
mod mach_o;
#[cfg(target_os = "macos")]
mod tag;

pub(crate) use self::codename::Codename;
#[cfg(target_os = "macos")]
pub(crate) use self::{codesign::Codesign, mach_o::MachO, tag::Tag};
