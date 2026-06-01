use std::str::FromStr;

use anyhow::anyhow;
use os_info::Version;
use serde_with::DeserializeFromStr;

use super::super::semver::Semver;
use crate::context::INFO;

#[derive(PartialEq, Eq, PartialOrd, Ord, DeserializeFromStr)]
pub(crate) enum Codename {
    Catalina,
    BigSur,
    Monterey,
    Ventura,
    Sonoma,
    Sequoia,
    Tahoe,
}

impl Codename {
    pub(crate) fn try_default() -> anyhow::Result<Self> {
        let info = &INFO;

        let version = info.version();

        let &Version::Semantic(major, minor, patch) = version else {
            let err = anyhow!(r#"Unsupported macOS version detected: "{version}""#);

            return Err(err);
        };

        let semver = Semver {
            major,
            minor: Some(minor),
            patch: Some(patch),
        };

        let this = Self::try_from(semver)?;

        Ok(this)
    }
}

impl TryFrom<Semver> for Codename {
    type Error = CodenameError;

    fn try_from(semver: Semver) -> Result<Self, Self::Error> {
        let this = match (semver.major, semver.minor, semver.patch) {
            (26, ..) => Self::Tahoe,
            (15, ..) => Self::Sequoia,
            (14, ..) => Self::Sonoma,
            (13, ..) => Self::Ventura,
            (12, ..) => Self::Monterey,
            (11, ..) => Self::BigSur,
            (10, Some(15), _) => Self::Catalina,
            _ => return Err(CodenameError::Unsupported),
        };

        Ok(this)
    }
}

impl FromStr for Codename {
    type Err = CodenameError;

    fn from_str(codename: &str) -> Result<Self, Self::Err> {
        let this = match codename {
            "tahoe" | "26" => Self::Tahoe,
            "sequoia" | "15" => Self::Sequoia,
            "sonoma" | "14" => Self::Sonoma,
            "ventura" | "13" => Self::Ventura,
            "monterey" | "12" => Self::Monterey,
            "big_sur" | "11" => Self::BigSur,
            "catalina" | "10.15" => Self::Catalina,
            _ => return Err(CodenameError::Unsupported),
        };

        Ok(this)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CodenameError {
    #[error("Unsupported macOS codename detected")]
    Unsupported,
}

impl CodenameError {
    pub(super) fn unsupported_into_none(self) -> Option<anyhow::Error> {
        match self {
            Self::Unsupported => None,
        }
    }
}
