use super::{Compatible, CompatibleInner};
use crate::package::raw::{DependsOnLinux, DependsOnMaximumMacos, DependsOnMinimumMacos};

pub(crate) struct Compatibility;

impl Compatible for Compatibility {
    fn try_new() -> anyhow::Result<Self> {
        let this = Self;

        Ok(this)
    }
}

impl CompatibleInner for Compatibility {
    fn check_os(
        &self,
        minimum_macos: Option<&DependsOnMinimumMacos>,
        maximum_macos: Option<&DependsOnMaximumMacos>,
        linux: Option<&DependsOnLinux>,
    ) -> bool {
        if self.check_linux(linux) {
            return true;
        }

        if !self.check_minimum_macos(minimum_macos) {
            return false;
        }

        if !self.check_maximum_macos(maximum_macos) {
            return false;
        }

        true
    }

    fn check_minimum_macos(&self, minimum_macos: Option<&DependsOnMinimumMacos>) -> bool {
        minimum_macos.is_none()
    }

    fn check_maximum_macos(&self, maximum_macos: Option<&DependsOnMaximumMacos>) -> bool {
        maximum_macos.is_none()
    }
}
