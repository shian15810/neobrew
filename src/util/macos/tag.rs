use std::{cmp::Ordering, result, str::FromStr};

use anyhow::{Result, anyhow};
use oci_client::config::Architecture;
use os_info::Version;

use super::{codename::Codename, semver::Semver};

#[derive(PartialEq, Eq)]
pub(crate) struct Tag {
    architecture: Architecture,
    codename: Codename,
}

impl Tag {
    pub(crate) fn architecture(&self) -> &Architecture {
        &self.architecture
    }

    pub(crate) fn try_default() -> Result<Self> {
        let architecture = Architecture::default();

        let info = os_info::get();

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

        let this = match Self::try_from((architecture, semver)) {
            Ok(this) => this,
            Err(Some(err)) => return Err(err),
            Err(None) => {
                let err = anyhow!(r#"Unsupported macOS semver detected: "{version}""#);

                return Err(err);
            },
        };

        Ok(this)
    }
}

impl TryFrom<(Architecture, Semver)> for Tag {
    type Error = Option<anyhow::Error>;

    fn try_from(
        (architecture, semver): (Architecture, Semver),
    ) -> result::Result<Self, Self::Error> {
        let codename = Codename::try_from(semver)?;

        let this = Self {
            architecture,
            codename,
        };

        Ok(this)
    }
}

impl FromStr for Tag {
    type Err = Option<anyhow::Error>;

    fn from_str(tag: &str) -> result::Result<Self, Self::Err> {
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
        self.codename.cmp(&other.codename).then_with(|| {
            self.architecture
                .to_string()
                .cmp(&other.architecture.to_string())
        })
    }
}
