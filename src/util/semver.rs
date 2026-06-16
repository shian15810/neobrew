use std::{cmp::Ordering, str::FromStr};

use anyhow::Context as _;

pub(super) struct Semver {
    pub(super) major: u64,
    pub(super) minor: Option<u64>,
    pub(super) patch: Option<u64>,
}

impl FromStr for Semver {
    type Err = anyhow::Error;

    fn from_str(semver: &str) -> Result<Self, Self::Err> {
        let mut parts = semver.split('.');

        let major = parts.next().context("Semver string is empty")?;
        let major = major.parse::<u64>()?;

        #[expect(clippy::redundant_closure_for_method_calls)]
        let minor = parts.next().map(|minor| minor.parse::<u64>()).transpose()?;

        #[expect(clippy::redundant_closure_for_method_calls)]
        let patch = parts.next().map(|patch| patch.parse::<u64>()).transpose()?;

        let this = Self {
            major,
            minor,
            patch,
        };

        Ok(this)
    }
}

impl PartialEq for Semver {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor.unwrap_or(0) == other.minor.unwrap_or(0)
            && self.patch.unwrap_or(0) == other.patch.unwrap_or(0)
    }
}

impl Eq for Semver {}

impl PartialOrd for Semver {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Semver {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.unwrap_or(0).cmp(&other.minor.unwrap_or(0)))
            .then_with(|| self.patch.unwrap_or(0).cmp(&other.patch.unwrap_or(0)))
    }
}
