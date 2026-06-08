#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use std::sync::Arc;

#[cfg(target_os = "linux")]
pub(crate) use self::linux::Artifact;
#[cfg(target_os = "macos")]
pub(crate) use self::macos::Artifact;
use crate::{context::Context, package::prepared::PreparedCask, placeholder::Placeholder};

pub(crate) trait Artifactable {
    fn new(placeholder: Arc<Placeholder>, context: Arc<Context>) -> Self;

    async fn relocate(&self, prepared_cask: &PreparedCask) -> anyhow::Result<()>;
}
