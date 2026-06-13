use std::{cmp::Ordering, str::FromStr};

use oci_client::config::Architecture;
use thiserror::Error;

use super::{
    super::semver::Semver,
    codename::{Codename, CodenameError},
};

#[derive(PartialEq, Eq)]
pub(crate) struct Tag {
    architecture: Architecture,
    codename: Codename,
}

impl Tag {
    pub(crate) fn architecture(&self) -> &Architecture {
        &self.architecture
    }

    pub(crate) fn try_default() -> anyhow::Result<Self> {
        let architecture = Architecture::default();

        let codename = Codename::try_default()?;

        let this = Self::from((architecture, codename));

        Ok(this)
    }
}

impl From<(Architecture, Codename)> for Tag {
    fn from((architecture, codename): (Architecture, Codename)) -> Self {
        Self {
            architecture,
            codename,
        }
    }
}

impl TryFrom<(Architecture, Semver)> for Tag {
    type Error = TagError;

    fn try_from((architecture, semver): (Architecture, Semver)) -> Result<Self, Self::Error> {
        let codename = Codename::try_from(semver)?;

        let this = Self {
            architecture,
            codename,
        };

        Ok(this)
    }
}

impl FromStr for Tag {
    type Err = TagError;

    fn from_str(tag: &str) -> Result<Self, Self::Err> {
        let (codename, architecture) = match tag.strip_prefix("arm64_") {
            Some(codename) => (codename, Architecture::ARM64),
            None => (tag, Architecture::Amd64),
        };

        let codename = codename.parse::<Codename>()?;

        let this = Self {
            architecture,
            codename,
        };

        Ok(this)
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tag {
    fn cmp(&self, other: &Self) -> Ordering {
        self.codename.cmp(&other.codename)
    }
}

#[derive(Debug, Error)]
pub(crate) enum TagError {
    #[error("Unsupported macOS tag detected")]
    Unsupported,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<CodenameError> for TagError {
    fn from(codename_error: CodenameError) -> Self {
        match codename_error {
            CodenameError::Unsupported => Self::Unsupported,
            CodenameError::Other(err) => Self::Other(err),
        }
    }
}
