use std::str::FromStr;

use super::semver::Semver;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Codename {
    Catalina,
    BigSur,
    Monterey,
    Ventura,
    Sonoma,
    Sequoia,
    Tahoe,
}

impl TryFrom<Semver> for Codename {
    type Error = Option<anyhow::Error>;

    fn try_from(semver: Semver) -> Result<Self, Self::Error> {
        let this = match (semver.major, semver.minor, semver.patch) {
            (26, ..) => Self::Tahoe,
            (15, ..) => Self::Sequoia,
            (14, ..) => Self::Sonoma,
            (13, ..) => Self::Ventura,
            (12, ..) => Self::Monterey,
            (11, ..) => Self::BigSur,
            (10, Some(15), _) => Self::Catalina,
            _ => return Err(None),
        };

        Ok(this)
    }
}

impl FromStr for Codename {
    type Err = Option<anyhow::Error>;

    fn from_str(codename: &str) -> Result<Self, Self::Err> {
        let this = match codename {
            "tahoe" => Self::Tahoe,
            "sequoia" => Self::Sequoia,
            "sonoma" => Self::Sonoma,
            "ventura" => Self::Ventura,
            "monterey" => Self::Monterey,
            "big_sur" => Self::BigSur,
            "catalina" => Self::Catalina,
            _ => return Err(None),
        };

        Ok(this)
    }
}
