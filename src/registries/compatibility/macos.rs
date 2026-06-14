use super::{CaskCompatibilityInner, Compatibility, CompatibilityExt, FormulaCompatibilityInner};
use crate::{
    context::Context,
    package::raw::cask::{DependsOnLinux, DependsOnMaximumMacos, DependsOnMinimumMacos},
    util::macos::{codename::Codename, xcode::Xcode},
};

impl CompatibilityExt for Compatibility {
    async fn try_new(context: &Context) -> anyhow::Result<Self> {
        let this = Self {
            codename: Codename::try_default(context)?,

            xcode: Xcode::try_default().await?,
        };

        Ok(this)
    }
}

impl FormulaCompatibilityInner for Compatibility {
    fn check_requirement_minimum_xcode(&self, version: Option<&str>) -> anyhow::Result<bool> {
        let Some(version) = version else {
            return Ok(true);
        };

        let minimum_xcode = version.parse::<Xcode>()?;

        let is_compatible = self.xcode >= minimum_xcode;

        Ok(is_compatible)
    }

    fn check_requirement_minimum_macos(&self, version: Option<&str>) -> anyhow::Result<bool> {
        let Some(version) = version else {
            return Ok(true);
        };

        let minimum_codename = version.parse::<Codename>()?;

        let is_compatible = self.codename >= minimum_codename;

        Ok(is_compatible)
    }

    fn check_requirement_maximum_macos(&self, version: Option<&str>) -> anyhow::Result<bool> {
        let Some(version) = version else {
            return Ok(true);
        };

        let maximum_codename = version.parse::<Codename>()?;

        let is_compatible = self.codename <= maximum_codename;

        Ok(is_compatible)
    }
}

impl CaskCompatibilityInner for Compatibility {
    fn check_depends_on_os(
        &self,
        minimum_macos: Option<&DependsOnMinimumMacos>,
        maximum_macos: Option<&DependsOnMaximumMacos>,
        linux: Option<&DependsOnLinux>,
    ) -> bool {
        if self.check_depends_on_linux(linux) {
            return false;
        }

        if !self.check_depends_on_minimum_macos(minimum_macos) {
            return false;
        }

        if !self.check_depends_on_maximum_macos(maximum_macos) {
            return false;
        }

        true
    }

    fn check_depends_on_minimum_macos(
        &self,
        minimum_macos: Option<&DependsOnMinimumMacos>,
    ) -> bool {
        let Some(minimum_macos) = minimum_macos else {
            return true;
        };

        let Some(minimum_codename) = minimum_macos.codenames.iter().max() else {
            return true;
        };

        &self.codename >= minimum_codename
    }

    fn check_depends_on_maximum_macos(
        &self,
        maximum_macos: Option<&DependsOnMaximumMacos>,
    ) -> bool {
        let Some(maximum_macos) = maximum_macos else {
            return true;
        };

        let Some(maximum_codename) = maximum_macos.codenames.iter().min() else {
            return true;
        };

        &self.codename <= maximum_codename
    }
}
