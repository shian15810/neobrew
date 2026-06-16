use std::collections::HashMap;

use os_info::Bitness;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::Deserialize_repr;
use serde_with::{BoolFromInt, FromInto, serde_as};

use super::{super::PackageExt, RawPackageExt};
use crate::{ext::serde::true_on_absent, util::macos::codename::Codename};

#[derive(Deserialize)]
pub(crate) struct RawCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
    pub(in super::super) artifacts: Vec<Artifact>,
    pub(in super::super) variations: HashMap<String, Variation>,

    depends_on: DependsOn,
}

impl PackageExt for RawCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl RawPackageExt for RawCask {}

impl RawCask {
    pub(crate) fn depends_on(&self) -> &DependsOn {
        &self.depends_on
    }

    pub(crate) fn dependencies(&self) -> &[String] {
        &self.depends_on.cask
    }

    pub(crate) fn formula_dependencies(&self) -> &[String] {
        &self.depends_on.formula
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(in super::super) enum Artifact {
    App {
        app: ArtifactCommonSource,
        target: String,
    },
    Suite {
        suite: ArtifactCommonSource,
        target: String,
    },
    Pkg {
        pkg: ArtifactPkgSource,
    },
    Installer {
        installer: Vec<ArtifactInstallerSource>,
    },
    Binary {
        binary: ArtifactCommonSource,
        target: String,
    },
    Manpage {
        manpage: ArtifactCommonSource,
        target: String,
    },
    BashCompletion {
        bash_completion: ArtifactCommonSource,
        target: String,
    },
    FishCompletion {
        fish_completion: ArtifactCommonSource,
        target: String,
    },
    ZshCompletion {
        zsh_completion: ArtifactCommonSource,
        target: String,
    },
    GenerateCompletionsFromExecutable {
        generate_completions_from_executable: ArtifactGenerateCompletionsFromExecutableSource,
    },
    Colorpicker {
        colorpicker: ArtifactCommonSource,
        target: String,
    },
    Dictionary {
        dictionary: ArtifactCommonSource,
        target: String,
    },
    Font {
        font: ArtifactCommonSource,
        target: String,
    },
    InputMethod {
        input_method: ArtifactCommonSource,
        target: String,
    },
    InternetPlugin {
        internet_plugin: ArtifactCommonSource,
        target: String,
    },
    KeyboardLayout {
        keyboard_layout: ArtifactCommonSource,
        target: String,
    },
    Prefpane {
        prefpane: ArtifactCommonSource,
        target: String,
    },
    Qlplugin {
        qlplugin: ArtifactCommonSource,
        target: String,
    },
    Mdimporter {
        mdimporter: ArtifactCommonSource,
        target: String,
    },
    ScreenSaver {
        screen_saver: ArtifactCommonSource,
        target: String,
    },
    Service {
        service: ArtifactCommonSource,
        target: String,
    },
    AudioUnitPlugin {
        audio_unit_plugin: ArtifactCommonSource,
        target: String,
    },
    VstPlugin {
        vst_plugin: ArtifactCommonSource,
        target: String,
    },
    Vst3Plugin {
        vst3_plugin: ArtifactCommonSource,
        target: String,
    },
    #[expect(clippy::enum_variant_names)]
    Artifact {
        artifact: ArtifactCommonSource,
        target: String,
    },
    StageOnly {
        stage_only: ArtifactStageOnlySource,
    },
    Unsupported(HashMap<String, Value>),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(in super::super) enum ArtifactCommonSource {
    WithOptions(String, ArtifactCommonSourceOptions),
    WithoutOptions((String,)),
}

#[derive(Deserialize)]
pub(in super::super) struct ArtifactCommonSourceOptions {
    pub(in super::super) target: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(in super::super) enum ArtifactPkgSource {
    WithOptions(String, ArtifactPkgSourceOptions),
    WithoutOptions((String,)),
}

#[derive(Deserialize)]
pub(in super::super) struct ArtifactPkgSourceOptions {
    #[serde(default)]
    pub(in super::super) allow_untrusted: bool,
    #[serde(default)]
    pub(in super::super) choices: Vec<ArtifactPkgSourceOptionsChoice>,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub(crate) struct ArtifactPkgSourceOptionsChoice {
    #[serde(rename = "choiceIdentifier")]
    identifier: String,
    #[serde(rename = "choiceAttribute")]
    attribute: ArtifactPkgSourceOptionsChoiceAttribute,
    #[serde(rename = "attributeSetting")]
    #[serde_as(as = "BoolFromInt")]
    attribute_setting: bool,
}

#[derive(Serialize, Deserialize)]
enum ArtifactPkgSourceOptionsChoiceAttribute {
    #[serde(rename = "selected")]
    Selected,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(in super::super) enum ArtifactInstallerSource {
    Manual {
        manual: String,
    },
    Script {
        script: ArtifactInstallerSourceScript,
    },
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Deserialize)]
pub(in super::super) struct ArtifactInstallerSourceScript {
    executable: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    input: Vec<String>,
    #[serde(default = "true_on_absent")]
    must_succeed: bool,
    #[serde(default = "true_on_absent")]
    print_stderr: bool,
    #[serde(default = "true_on_absent")]
    print_stdout: bool,
    #[serde(default)]
    sudo: bool,
}

#[derive(Deserialize)]
pub(in super::super) struct ArtifactGenerateCompletionsFromExecutableSource(
    pub(in super::super) String,
    pub(in super::super) String,
    pub(in super::super) ArtifactGenerateCompletionsFromExecutableSourceOptions,
);

#[derive(Deserialize)]
pub(in super::super) struct ArtifactGenerateCompletionsFromExecutableSourceOptions {
    pub(in super::super) base_name: Option<String>,
    pub(in super::super) shell_parameter_format: Option<String>,
    pub(in super::super) shells: Vec<String>,
}

#[derive(Deserialize)]
pub(in super::super) struct ArtifactStageOnlySource(pub(in super::super) (bool,));

#[derive(Deserialize)]
pub(in super::super) struct Variation<Artifacts = Option<Vec<Artifact>>> {
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
    pub(in super::super) artifacts: Artifacts,
}

#[derive(Deserialize)]
pub(crate) struct DependsOn {
    #[serde(default)]
    formula: Vec<String>,
    #[serde(default)]
    cask: Vec<String>,
    #[serde(rename = "macos", default)]
    pub(crate) minimum_macos: Option<DependsOnMinimumMacos>,
    #[serde(default)]
    pub(crate) maximum_macos: Option<DependsOnMaximumMacos>,
    #[serde(default)]
    pub(crate) linux: Option<DependsOnLinux>,
    #[serde(rename = "arch", default)]
    pub(crate) arches: Vec<DependsOnArch>,
}

#[derive(Deserialize)]
pub(crate) struct DependsOnMinimumMacos {
    #[serde(rename = ">=", default)]
    pub(crate) codenames: Vec<Codename>,
}

#[derive(Deserialize)]
pub(crate) struct DependsOnMaximumMacos {
    #[serde(rename = "<=", default)]
    pub(crate) codenames: Vec<Codename>,
}

#[derive(Deserialize)]
pub(crate) struct DependsOnLinux;

#[serde_as]
#[derive(Deserialize)]
pub(crate) struct DependsOnArch {
    #[serde(rename = "type")]
    pub(crate) brand: DependsOnArchBrand,
    #[serde_as(as = "FromInto<DependsOnArchBits>")]
    pub(crate) bits: Bitness,
}

#[derive(Deserialize)]
pub(crate) enum DependsOnArchBrand {
    #[serde(rename = "arm")]
    Arm,
    #[serde(rename = "intel")]
    Intel,
}

#[derive(Deserialize_repr)]
#[repr(u8)]
enum DependsOnArchBits {
    SixtyFour = 64,
}

impl From<DependsOnArchBits> for Bitness {
    fn from(bits: DependsOnArchBits) -> Self {
        match bits {
            DependsOnArchBits::SixtyFour => Self::X64,
        }
    }
}
