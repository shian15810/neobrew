use super::{Compatible, CompatibleInner};
use crate::{
    package::raw::{DependsOnLinux, DependsOnMaximumMacos, DependsOnMinimumMacos},
    util::macos,
};

pub(crate) struct Compatibility {
    codename: macos::Codename,
}

impl Compatible for Compatibility {
    fn try_new() -> anyhow::Result<Self> {
        let this = Self {
            codename: macos::Codename::try_default()?,
        };

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
            return false;
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
        let Some(minimum_macos) = minimum_macos else {
            return true;
        };

        let Some(minimum_codename) = minimum_macos.codenames.iter().max() else {
            return true;
        };

        &self.codename >= minimum_codename
    }

    fn check_maximum_macos(&self, maximum_macos: Option<&DependsOnMaximumMacos>) -> bool {
        let Some(maximum_macos) = maximum_macos.as_ref() else {
            return true;
        };

        let Some(maximum_codename) = maximum_macos.codenames.iter().min() else {
            return true;
        };

        &self.codename <= maximum_codename
    }
}
