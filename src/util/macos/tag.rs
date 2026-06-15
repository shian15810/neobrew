use std::{cmp::Ordering, str::FromStr};

use oci_client::config::Architecture;
use thiserror::Error;

use super::{
    super::semver::Semver,
    codename::{Codename, CodenameError},
};
use crate::context::Context;

#[derive(PartialEq, Eq)]
pub(crate) struct Tag {
    codename: Codename,
    architecture: Architecture,
}

impl From<(Codename, Architecture)> for Tag {
    fn from((codename, architecture): (Codename, Architecture)) -> Self {
        Self {
            codename,
            architecture,
        }
    }
}

impl TryFrom<(Semver, Architecture)> for Tag {
    type Error = TagError;

    fn try_from((semver, architecture): (Semver, Architecture)) -> Result<Self, Self::Error> {
        let codename = Codename::try_from(semver)?;

        let this = Self {
            codename,
            architecture,
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
            codename,
            architecture,
        };

        Ok(this)
    }
}

impl Tag {
    pub(crate) fn try_default(context: &Context) -> anyhow::Result<Self> {
        let codename = Codename::try_default(context)?;

        let architecture = Architecture::default();

        let this = Self::from((codename, architecture));

        Ok(this)
    }

    pub(crate) fn architecture(&self) -> &Architecture {
        &self.architecture
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tag {
    fn cmp(&self, other: &Self) -> Ordering {
        self.codename.cmp(&other.codename).then_with(|| {
            self.architecture
                .to_string()
                .cmp(&other.architecture.to_string())
        })
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
