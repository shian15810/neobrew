use std::collections::HashMap;

use os_info::Bitness;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::Deserialize_repr;
use serde_with::{BoolFromInt, DeserializeFromStr, FromInto, serde_as};
use strum::{AsRefStr, Display, EnumString};

use super::{super::PackageExt, RawPackageExt};
use crate::{context::Context, ext::serde::true_on_absent, util::macos::codename::Codename};

#[derive(Deserialize)]
pub(crate) struct RawCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    pub(in super::super) url: String,
    pub(in super::super) sha256: String,
    pub(in super::super) artifacts: Vec<Artifact>,

    depends_on: DependsOn,

    variations: HashMap<String, Variation>,
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
    Completions {
        #[serde(rename = "generate_completions_from_executable")]
        completions: ArtifactCompletionsSource,
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
pub(crate) enum ArtifactInstallerSource {
    Manual {
        manual: String,
    },
    Script {
        script: ArtifactInstallerSourceScript,
    },
}

#[expect(clippy::struct_excessive_bools)]
#[derive(Deserialize)]
pub(crate) struct ArtifactInstallerSourceScript {
    #[serde(default)]
    pub(crate) sudo: bool,
    pub(crate) executable: String,
    #[serde(default)]
    pub(crate) args: Vec<String>,
    #[serde(default = "true_on_absent")]
    print_stdout: bool,
    #[serde(default = "true_on_absent")]
    print_stderr: bool,
    #[serde(default)]
    pub(crate) input: Vec<String>,
    #[serde(default = "true_on_absent")]
    pub(crate) must_succeed: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(in super::super) enum ArtifactCompletionsSource {
    WithSubcommand(String, String, ArtifactCompletionsSourceOptions),
    WithoutSubcommand(String, ArtifactCompletionsSourceOptions),
}

#[derive(Deserialize)]
pub(in super::super) struct ArtifactCompletionsSourceOptions {
    #[serde(rename = "base_name")]
    pub(in super::super) name: Option<String>,
    #[serde(rename = "shell_parameter_format")]
    pub(in super::super) format: Option<ArtifactCompletionsSourceOptionsFormat>,
    pub(in super::super) shells: Vec<ArtifactCompletionsSourceOptionsShell>,
}

#[derive(Debug, EnumString, DeserializeFromStr)]
pub(crate) enum ArtifactCompletionsSourceOptionsFormat {
    #[strum(to_string = "arg")]
    Arg,
    #[strum(to_string = "clap")]
    Clap,
    #[strum(to_string = "click")]
    Click,
    #[strum(to_string = "cobra")]
    Cobra,
    #[strum(to_string = "flag")]
    Flag,
    #[strum(to_string = "none")]
    None,
    #[strum(to_string = "typer")]
    Typer,
    #[strum(default)]
    Other(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Display, AsRefStr, EnumString, DeserializeFromStr)]
pub(crate) enum ArtifactCompletionsSourceOptionsShell {
    #[strum(to_string = "bash")]
    Bash,
    #[strum(to_string = "fish")]
    Fish,
    #[strum(to_string = "zsh")]
    Zsh,
    #[strum(to_string = "pwsh")]
    Pwsh,
}

#[derive(Deserialize)]
pub(in super::super) struct ArtifactStageOnlySource(pub(in super::super) (bool,));

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

#[expect(clippy::empty_structs_with_brackets)]
#[derive(Deserialize)]
pub(crate) struct DependsOnLinux {}

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
struct Variation {
    version: Option<String>,
    url: Option<String>,
    sha256: Option<String>,
    artifacts: Option<Vec<Artifact>>,

    depends_on: Option<DependsOn>,
}

impl RawCask {
    pub(crate) fn squash_variations(mut self, context: &Context) -> anyhow::Result<Self> {
        #[expect(clippy::collapsible_if)]
        if let Some(variation_key) = self.variation_key(context)? {
            if let Some(variation) = self.variations.remove(&variation_key) {
                if let Some(version) = variation.version {
                    self.version = version;
                }

                if let Some(url) = variation.url {
                    self.url = url;
                }

                if let Some(sha256) = variation.sha256 {
                    self.sha256 = sha256;
                }

                if let Some(artifacts) = variation.artifacts {
                    self.artifacts = artifacts;
                }

                if let Some(depends_on) = variation.depends_on {
                    self.depends_on = depends_on;
                }
            }
        }

        self.variations.clear();

        self.variations.shrink_to_fit();

        Ok(self)
    }

    #[cfg(target_os = "macos")]
    fn variation_key(&self, context: &Context) -> anyhow::Result<Option<String>> {
        use crate::util::macos::tag::{Tag, TagError};

        let current_tag = Tag::try_default(context)?;

        #[cfg(debug_assertions)]
        let variation_keys_tags = self
            .variations
            .keys()
            .filter_map(|variation_key| {
                let variation_tag = match variation_key.parse::<Tag>() {
                    Ok(variation_tag) => variation_tag,
                    Err(TagError::Unsupported) => return None,
                    Err(TagError::Other(err)) => return Some(Err(err)),
                };

                Some(Ok((variation_key, variation_tag)))
            })
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let variation_keys_tags = self
            .variations
            .keys()
            .filter_map(|variation_key| {
                let variation_tag = match variation_key.parse::<Tag>() {
                    Ok(variation_tag) => variation_tag,
                    Err(TagError::Unsupported) => return None,
                    Err(TagError::Other(err)) => return Some(Err(err)),
                };

                Some(Ok((variation_key, variation_tag)))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let variation_key = variation_keys_tags
            .into_iter()
            .filter(|(_, variation_tag)| {
                let is_macos_architecture_equal =
                    variation_tag.architecture() == current_tag.architecture();

                is_macos_architecture_equal && variation_tag <= &current_tag
            })
            .max_by(|(_, left), (_, right)| left.cmp(right))
            .map(|(variation_key, _)| variation_key.to_owned());

        Ok(variation_key)
    }

    #[cfg(target_os = "linux")]
    #[expect(clippy::unnecessary_wraps)]
    fn variation_key(&self, _context: &Context) -> anyhow::Result<Option<String>> {
        let variation_key = cfg_select! {
            target_arch = "aarch64" => "arm64_linux",
            target_arch = "x86_64" => "x86_64_linux",
        };
        let variation_key = self
            .variations
            .contains_key(variation_key)
            .then(|| variation_key.to_owned());

        Ok(variation_key)
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use serde_json::json;

    use super::*;

    #[test]
    fn deserialize_artifact_completions_source_options_format() {
        let json = json!("clap");

        let format: ArtifactCompletionsSourceOptionsFormat = serde_json::from_value(json).unwrap();

        assert_matches!(format, ArtifactCompletionsSourceOptionsFormat::Clap);

        let json = json!("--autocomplete=init:");

        let format: ArtifactCompletionsSourceOptionsFormat = serde_json::from_value(json).unwrap();

        assert_matches!(
            format,
            ArtifactCompletionsSourceOptionsFormat::Other(other) if other == "--autocomplete=init:",
        );
    }
}
