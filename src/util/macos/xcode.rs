use std::str::FromStr;

use anyhow::{Context as _, anyhow};
use tokio::process::Command;

use super::super::semver::Semver;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Xcode {
    semver: Semver,
    build: Option<String>,
}

impl FromStr for Xcode {
    type Err = anyhow::Error;

    fn from_str(xcode: &str) -> Result<Self, Self::Err> {
        let semver = xcode;

        if let Ok(semver) = semver.parse::<Semver>() {
            let this = Self {
                semver,
                build: None,
            };

            return Ok(this);
        }

        let stdout = xcode;

        let mut stdout_lines = stdout.lines();

        let semver_line = stdout_lines.next().context("Xcode stdout is empty")?;

        let semver = semver_line
            .strip_prefix("Xcode ")
            .context("Xcode semver line is invalid")?;
        let semver = semver.parse::<Semver>()?;

        let build_line = stdout_lines.next().context("Xcode build line is missing")?;

        #[expect(clippy::redundant_closure_for_method_calls)]
        let build = build_line
            .strip_prefix("Build version ")
            .map(|build| build.to_owned());

        let this = Self {
            semver,
            build,
        };

        Ok(this)
    }
}

impl Xcode {
    pub(crate) async fn try_default() -> anyhow::Result<Self> {
        let mut xcodebuild = Command::new("xcodebuild");

        xcodebuild.arg("-version");

        let xcodebuild = xcodebuild.output().await?;

        if !xcodebuild.status.success() {
            let stdout = String::from_utf8_lossy(&xcodebuild.stdout);

            let stderr = String::from_utf8_lossy(&xcodebuild.stderr);

            let err = anyhow!("{stdout}{stderr}");

            return Err(err);
        }

        let stdout = String::from_utf8_lossy(&xcodebuild.stdout);

        let this = stdout.parse::<Self>()?;

        Ok(this)
    }
}
