#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use os_info::Bitness;

use crate::package::raw::{
    cask::{
        DependsOn,
        DependsOnArch,
        DependsOnArchBrand,
        DependsOnLinux,
        DependsOnMaximumMacos,
        DependsOnMinimumMacos,
        RawCask,
    },
    formula::{RawFormula, Requirement, RequirementName, RequirementSpec},
};
#[cfg(target_os = "macos")]
use crate::util::macos::{codename::Codename, xcode::Xcode};

pub(super) struct Compatibility {
    #[cfg(target_os = "macos")]
    codename: Codename,

    #[cfg(target_os = "macos")]
    xcode: Xcode,
}

pub(super) trait CompatibilityExt: FormulaCompatibility + CaskCompatibility + Sized {
    async fn try_new() -> anyhow::Result<Self>;
}

#[expect(private_bounds)]
pub(super) trait FormulaCompatibility: FormulaCompatibilityInner {
    fn is_formula_compatible(&self, raw_formula: &RawFormula) -> anyhow::Result<bool> {
        let requirements = raw_formula.requirements();

        let is_compatible = self.check_requirements(requirements)?;

        Ok(is_compatible)
    }
}

impl<FormulaCompatInner: FormulaCompatibilityInner> FormulaCompatibility for FormulaCompatInner {}

trait FormulaCompatibilityInner {
    fn check_requirements(&self, requirements: &[Requirement]) -> anyhow::Result<bool> {
        #[cfg(debug_assertions)]
        let are_compatible = requirements
            .iter()
            .filter(|requirement| {
                requirement.contexts.is_empty()
                    && requirement.specs.contains(&RequirementSpec::Stable)
            })
            .map(|requirement| self.check_requirement(requirement))
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let are_compatible = requirements
            .iter()
            .filter(|requirement| {
                requirement.contexts.is_empty()
                    && requirement.specs.contains(&RequirementSpec::Stable)
            })
            .map(|requirement| self.check_requirement(requirement))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let is_compatible = are_compatible
            .into_iter()
            .all(|is_compatible| is_compatible);

        Ok(is_compatible)
    }

    fn check_requirement(&self, requirement: &Requirement) -> anyhow::Result<bool> {
        let version = requirement.version.as_deref();

        let is_compatible = match requirement.name {
            RequirementName::MinimumXcode => self.check_requirement_minimum_xcode(version)?,
            RequirementName::MinimumMacos => self.check_requirement_minimum_macos(version)?,
            RequirementName::MaximumMacos => self.check_requirement_maximum_macos(version)?,
            RequirementName::Linux => self.check_requirement_linux(),
            RequirementName::Arch => self.check_requirement_arch(version),
            RequirementName::Unsupported(_) => true,
        };

        Ok(is_compatible)
    }

    fn check_requirement_minimum_xcode(&self, version: Option<&str>) -> anyhow::Result<bool>;

    fn check_requirement_minimum_macos(&self, version: Option<&str>) -> anyhow::Result<bool>;

    fn check_requirement_maximum_macos(&self, version: Option<&str>) -> anyhow::Result<bool>;

    fn check_requirement_linux(&self) -> bool {
        cfg!(target_os = "linux")
    }

    fn check_requirement_arch(&self, version: Option<&str>) -> bool {
        match version {
            Some("arm64") => cfg!(target_arch = "aarch64"),
            Some("x86_64") => cfg!(target_arch = "x86_64"),
            _ => true,
        }
    }
}

impl<CaskCompatInner: CaskCompatibilityInner> CaskCompatibility for CaskCompatInner {}

#[expect(private_bounds)]
pub(super) trait CaskCompatibility: CaskCompatibilityInner {
    fn is_cask_compatible(&self, raw_cask: &RawCask) -> bool {
        let depends_on = raw_cask.depends_on();

        self.check_depends_on(depends_on)
    }
}

trait CaskCompatibilityInner {
    fn check_depends_on(&self, depends_on: &DependsOn) -> bool {
        let minimum_macos = depends_on.minimum_macos.as_ref();

        let maximum_macos = depends_on.maximum_macos.as_ref();

        let linux = depends_on.linux.as_ref();

        if !self.check_depends_on_os(minimum_macos, maximum_macos, linux) {
            return false;
        }

        if !self.check_depends_on_arch(&depends_on.arches) {
            return false;
        }

        true
    }

    fn check_depends_on_os(
        &self,
        minimum_macos: Option<&DependsOnMinimumMacos>,
        maximum_macos: Option<&DependsOnMaximumMacos>,
        linux: Option<&DependsOnLinux>,
    ) -> bool;

    fn check_depends_on_minimum_macos(&self, minimum_macos: Option<&DependsOnMinimumMacos>)
    -> bool;

    fn check_depends_on_maximum_macos(&self, maximum_macos: Option<&DependsOnMaximumMacos>)
    -> bool;

    fn check_depends_on_linux(&self, linux: Option<&DependsOnLinux>) -> bool {
        linux.is_some()
    }

    fn check_depends_on_arch(&self, arches: &[DependsOnArch]) -> bool {
        if arches.is_empty() {
            return true;
        }

        arches.iter().any(|arch| {
            let is_brand_compatible = match arch.brand {
                DependsOnArchBrand::Arm => cfg!(target_arch = "aarch64"),
                DependsOnArchBrand::Intel => cfg!(target_arch = "x86_64"),
            };

            let is_bits_compatible = matches!(arch.bits, Bitness::X64);

            is_brand_compatible && is_bits_compatible
        })
    }
}
