use std::{borrow::Cow, collections::HashMap};

use os_info::Bitness;
use serde::Deserialize;
use serde_json::Value;
use serde_repr::Deserialize_repr;
use serde_with::{BoolFromInt, FromInto, serde_as};

use super::{super::Packageable, RawPackageable};
use crate::{ext::serde::true_on_null, util::macos};

#[derive(Deserialize)]
pub(crate) struct RawCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
    pub(in super::super) artifacts: Vec<Artifact>,
    pub(in super::super) depends_on: DependsOn,
    pub(in super::super) variations: HashMap<String, Variation>,
}

impl Packageable for RawCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl RawPackageable for RawCask {
    fn version(&self) -> Cow<'_, str> {
        let version = &self.version;

        Cow::Borrowed(version)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(in super::super) enum Artifact {
    App {
        app: ArtifactSource,
        target: String,
    },
    Suite {
        suite: ArtifactSource,
        target: String,
    },
    Pkg {
        pkg: PkgSource,
    },
    Installer {
        installer: Vec<InstallerSource>,
    },
    Binary {
        binary: ArtifactSource,
        target: String,
    },
    Manpage {
        manpage: ArtifactSource,
        target: String,
    },
    BashCompletion {
        bash_completion: ArtifactSource,
        target: String,
    },
    FishCompletion {
        fish_completion: ArtifactSource,
        target: String,
    },
    ZshCompletion {
        zsh_completion: ArtifactSource,
        target: String,
    },
    GenerateCompletionsFromExecutable {
        generate_completions_from_executable: GenerateCompletionsFromExecutableSource,
    },
    Colorpicker {
        colorpicker: ArtifactSource,
        target: String,
    },
    Dictionary {
        dictionary: ArtifactSource,
        target: String,
    },
    Font {
        font: ArtifactSource,
        target: String,
    },
    InputMethod {
        input_method: ArtifactSource,
        target: String,
    },
    InternetPlugin {
        internet_plugin: ArtifactSource,
        target: String,
    },
    KeyboardLayout {
        keyboard_layout: ArtifactSource,
        target: String,
    },
    Prefpane {
        prefpane: ArtifactSource,
        target: String,
    },
    Mdimporter {
        mdimporter: ArtifactSource,
        target: String,
    },
    ScreenSaver {
        screen_saver: ArtifactSource,
        target: String,
    },
    Service {
        service: ArtifactSource,
        target: String,
    },
    AudioUnitPlugin {
        audio_unit_plugin: ArtifactSource,
        target: String,
    },
    VstPlugin {
        vst_plugin: ArtifactSource,
        target: String,
    },
    Vst3Plugin {
        vst3_plugin: ArtifactSource,
        target: String,
    },
    #[expect(clippy::enum_variant_names)]
    Artifact {
        artifact: ArtifactSource,
        target: String,
    },
    StageOnly {
        stage_only: (bool,),
    },
    Unsupported(HashMap<String, Value>),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(in super::super) enum ArtifactSource {
    WithOptions(String, ArtifactSourceOptions),
    WithoutOptions((String,)),
}

#[derive(Deserialize)]
pub(in super::super) struct ArtifactSourceOptions {
    target: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(in super::super) enum PkgSource {
    WithOptions(String, PkgSourceOptions),
    WithoutOptions((String,)),
}

#[derive(Deserialize)]
pub(in super::super) struct PkgSourceOptions {
    #[serde(default)]
    allow_untrusted: bool,
    #[serde(default)]
    choices: Vec<PkgSourceOptionsChoice>,
}

#[serde_as]
#[derive(Deserialize)]
struct PkgSourceOptionsChoice {
    #[serde(rename = "choiceIdentifier")]
    identifier: String,
    #[serde(rename = "choiceAttribute")]
    attribute: PkgSourceOptionsChoiceAttribute,
    #[serde(rename = "attributeSetting")]
    #[serde_as(as = "BoolFromInt")]
    attribute_setting: bool,
}

#[derive(Deserialize)]
enum PkgSourceOptionsChoiceAttribute {
    #[serde(rename = "selected")]
    Selected,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(in super::super) enum InstallerSource {
    Manual {
        manual: String,
    },
    Script {
        script: InstallerSourceScript,
    },
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Deserialize)]
pub(in super::super) struct InstallerSourceScript {
    executable: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    input: Vec<String>,
    #[serde(default = "true_on_null")]
    must_succeed: bool,
    #[serde(default = "true_on_null")]
    print_stderr: bool,
    #[serde(default = "true_on_null")]
    print_stdout: bool,
    #[serde(default)]
    sudo: bool,
}

#[derive(Deserialize)]
pub(in super::super) struct GenerateCompletionsFromExecutableSource(
    String,
    String,
    GenerateCompletionsFromExecutableSourceOptions,
);

#[derive(Deserialize)]
struct GenerateCompletionsFromExecutableSourceOptions {
    base_name: Option<String>,
    shell_parameter_format: Option<String>,
    shells: Vec<String>,
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
    pub(crate) codenames: Vec<macos::Codename>,
}

#[derive(Deserialize)]
pub(crate) struct DependsOnMaximumMacos {
    #[serde(rename = "<=", default)]
    pub(crate) codenames: Vec<macos::Codename>,
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

#[derive(Deserialize)]
pub(in super::super) struct Variation<Artifacts = Option<Vec<Artifact>>> {
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
    pub(in super::super) artifacts: Artifacts,
}
