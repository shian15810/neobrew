mod cask;
mod formula;

use enum_dispatch::enum_dispatch;

pub(super) use self::{
    cask::{
        Artifact,
        ArtifactCommonSource,
        ArtifactGenerateCompletionsFromExecutableSource,
        ArtifactInstallerSource,
        ArtifactPkgSource,
        ArtifactPkgSourceOptionsChoice,
        Variation,
    },
    formula::{Bottle, BottleStable, BottleStableFile, BottleStableFileCellar, Versions},
};
pub(crate) use self::{
    cask::{
        DependsOn,
        DependsOnArch,
        DependsOnArchBrand,
        DependsOnLinux,
        DependsOnMaximumMacos,
        DependsOnMinimumMacos,
        RawCask,
    },
    formula::RawFormula,
};
use super::Packageable;

#[enum_dispatch]
pub(crate) enum RawPackage {
    Formula(RawFormula),
    Cask(RawCask),
}

#[enum_dispatch(RawPackage)]
trait RawPackageable: Packageable {}
