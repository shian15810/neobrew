use super::{CaskCompatibilityInner, Compatibility, CompatibilityExt, FormulaCompatibilityInner};
use crate::package::raw::{DependsOnLinux, DependsOnMaximumMacos, DependsOnMinimumMacos};

impl CompatibilityExt for Compatibility {
    async fn try_new() -> anyhow::Result<Self> {
        let this = Self;

        Ok(this)
    }
}

impl FormulaCompatibilityInner for Compatibility {
    fn check_requirement_minimum_xcode(&self, _version: Option<&str>) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn check_requirement_minimum_macos(&self, version: Option<&str>) -> anyhow::Result<bool> {
        let is_compatible = version.is_some();

        Ok(is_compatible)
    }

    fn check_requirement_maximum_macos(&self, version: Option<&str>) -> anyhow::Result<bool> {
        let is_compatible = version.is_some();

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
            return true;
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
        minimum_macos.is_none()
    }

    fn check_depends_on_maximum_macos(
        &self,
        maximum_macos: Option<&DependsOnMaximumMacos>,
    ) -> bool {
        maximum_macos.is_none()
    }
}
