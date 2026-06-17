use anyhow::anyhow;
use os_info::Version;
use serde_with::DeserializeFromStr;
use strum::EnumString;
use thiserror::Error;

use super::super::semver::Semver;
use crate::context::Context;

#[derive(PartialEq, Eq, PartialOrd, Ord, EnumString, DeserializeFromStr)]
#[strum(
    parse_err_fn = CodenameError::unsupported,
    parse_err_ty = CodenameError
)]
pub(crate) enum Codename {
    #[strum(to_string = "catalina", serialize = "10.15")]
    Catalina,
    #[strum(to_string = "big_sur", serialize = "11")]
    BigSur,
    #[strum(to_string = "monterey", serialize = "12")]
    Monterey,
    #[strum(to_string = "ventura", serialize = "13")]
    Ventura,
    #[strum(to_string = "sonoma", serialize = "14")]
    Sonoma,
    #[strum(to_string = "sequoia", serialize = "15")]
    Sequoia,
    #[strum(to_string = "tahoe", serialize = "26")]
    Tahoe,
    #[strum(to_string = "golden_gate", serialize = "27")]
    GoldenGate,
}

impl TryFrom<Semver> for Codename {
    type Error = CodenameError;

    fn try_from(semver: Semver) -> Result<Self, Self::Error> {
        let this = match (semver.major, semver.minor, semver.patch) {
            (27, ..) => Self::GoldenGate,
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

impl Codename {
    pub(crate) fn try_default(context: &Context) -> anyhow::Result<Self> {
        let version = context.info.version();

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

#[derive(Debug, Error)]
pub(crate) enum CodenameError {
    #[error("Unsupported macOS codename detected")]
    Unsupported,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl CodenameError {
    fn unsupported(_: &str) -> Self {
        Self::Unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_codename() {
        assert!(Codename::GoldenGate > Codename::Catalina);

        assert!(Codename::Sonoma > Codename::Ventura);
    }
}
