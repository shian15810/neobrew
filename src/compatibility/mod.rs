#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use os_info::Bitness;

#[cfg(target_os = "linux")]
pub(crate) use self::linux::Compatibility;
#[cfg(target_os = "macos")]
pub(crate) use self::macos::Compatibility;
use crate::package::raw::{
    DependsOn,
    DependsOnArch,
    DependsOnArchBrand,
    DependsOnLinux,
    DependsOnMaximumMacos,
    DependsOnMinimumMacos,
};

#[expect(private_bounds)]
pub(crate) trait Compatible: CompatibleInner {
    fn try_new() -> anyhow::Result<Self>;

    fn check(&self, depends_on: &DependsOn) -> bool {
        let minimum_macos = depends_on.minimum_macos.as_ref();

        let maximum_macos = depends_on.maximum_macos.as_ref();

        let linux = depends_on.linux.as_ref();

        if !self.check_os(minimum_macos, maximum_macos, linux) {
            return false;
        }

        if !self.check_arch(&depends_on.arches) {
            return false;
        }

        true
    }
}

trait CompatibleInner: Sized {
    fn check_os(
        &self,
        minimum_macos: Option<&DependsOnMinimumMacos>,
        maximum_macos: Option<&DependsOnMaximumMacos>,
        linux: Option<&DependsOnLinux>,
    ) -> bool;

    fn check_minimum_macos(&self, minimum_macos: Option<&DependsOnMinimumMacos>) -> bool;

    fn check_maximum_macos(&self, maximum_macos: Option<&DependsOnMaximumMacos>) -> bool;

    fn check_linux(&self, linux: Option<&DependsOnLinux>) -> bool {
        linux.is_some()
    }

    fn check_arch(&self, arches: &[DependsOnArch]) -> bool {
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
