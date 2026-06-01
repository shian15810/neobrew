use super::super::raw::{
    Artifact,
    ArtifactCommonSource,
    ArtifactGenerateCompletionsFromExecutableSource,
    ArtifactInstallerSource,
    ArtifactPkgSource,
    ArtifactPkgSourceOptionsChoice,
};

#[derive(Default)]
pub(crate) struct Stanzas {
    pub(crate) app: Vec<CommonStanza>,
    pub(crate) suite: Vec<CommonStanza>,
    pkg: Vec<PkgStanza>,
    installer: Vec<ArtifactInstallerSource>,
    pub(crate) binary: Vec<CommonStanza>,
    pub(crate) manpage: Vec<CommonStanza>,
    pub(crate) bash_completion: Vec<CommonStanza>,
    pub(crate) fish_completion: Vec<CommonStanza>,
    pub(crate) zsh_completion: Vec<CommonStanza>,
    generate_completions_from_executable: Vec<GenerateCompletionsFromExecutableStanza>,
    pub(crate) colorpicker: Vec<CommonStanza>,
    pub(crate) dictionary: Vec<CommonStanza>,
    pub(crate) font: Vec<CommonStanza>,
    pub(crate) input_method: Vec<CommonStanza>,
    pub(crate) internet_plugin: Vec<CommonStanza>,
    pub(crate) keyboard_layout: Vec<CommonStanza>,
    pub(crate) prefpane: Vec<CommonStanza>,
    pub(crate) mdimporter: Vec<CommonStanza>,
    pub(crate) screen_saver: Vec<CommonStanza>,
    pub(crate) service: Vec<CommonStanza>,
    pub(crate) audio_unit_plugin: Vec<CommonStanza>,
    pub(crate) vst_plugin: Vec<CommonStanza>,
    pub(crate) vst3_plugin: Vec<CommonStanza>,
    pub(crate) artifact: Vec<CommonStanza>,
    stage_only: bool,
}

impl From<Vec<Artifact>> for Stanzas {
    #[expect(clippy::too_many_lines)]
    fn from(artifacts: Vec<Artifact>) -> Self {
        let mut this = Self::default();

        for artifact in artifacts {
            match artifact {
                Artifact::App {
                    app,
                    target,
                } => {
                    let app_stanza = CommonStanza::from((app, target));

                    this.app.push(app_stanza);
                },
                Artifact::Suite {
                    suite,
                    target,
                } => {
                    let suite_stanza = CommonStanza::from((suite, target));

                    this.suite.push(suite_stanza);
                },
                Artifact::Pkg {
                    pkg,
                } => {
                    let pkg_stanza = PkgStanza::from(pkg);

                    this.pkg.push(pkg_stanza);
                },
                Artifact::Installer {
                    installer,
                } => {
                    this.installer.extend(installer);
                },
                Artifact::Binary {
                    binary,
                    target,
                } => {
                    let binary_stanza = CommonStanza::from((binary, target));

                    this.binary.push(binary_stanza);
                },
                Artifact::Manpage {
                    manpage,
                    target,
                } => {
                    let manpage_stanza = CommonStanza::from((manpage, target));

                    this.manpage.push(manpage_stanza);
                },
                Artifact::BashCompletion {
                    bash_completion,
                    target,
                } => {
                    let bash_completion_stanza = CommonStanza::from((bash_completion, target));

                    this.bash_completion.push(bash_completion_stanza);
                },
                Artifact::FishCompletion {
                    fish_completion,
                    target,
                } => {
                    let fish_completion_stanza = CommonStanza::from((fish_completion, target));

                    this.fish_completion.push(fish_completion_stanza);
                },
                Artifact::ZshCompletion {
                    zsh_completion,
                    target,
                } => {
                    let zsh_completion_stanza = CommonStanza::from((zsh_completion, target));

                    this.zsh_completion.push(zsh_completion_stanza);
                },
                Artifact::GenerateCompletionsFromExecutable {
                    generate_completions_from_executable,
                } => {
                    let generate_completions_from_executable_stanza =
                        GenerateCompletionsFromExecutableStanza::from(
                            generate_completions_from_executable,
                        );

                    this.generate_completions_from_executable
                        .push(generate_completions_from_executable_stanza);
                },
                Artifact::Colorpicker {
                    colorpicker,
                    target,
                } => {
                    let colorpicker_stanza = CommonStanza::from((colorpicker, target));

                    this.colorpicker.push(colorpicker_stanza);
                },
                Artifact::Dictionary {
                    dictionary,
                    target,
                } => {
                    let dictionary_stanza = CommonStanza::from((dictionary, target));

                    this.dictionary.push(dictionary_stanza);
                },
                Artifact::Font {
                    font,
                    target,
                } => {
                    let font_stanza = CommonStanza::from((font, target));

                    this.font.push(font_stanza);
                },
                Artifact::InputMethod {
                    input_method,
                    target,
                } => {
                    let input_method_stanza = CommonStanza::from((input_method, target));

                    this.input_method.push(input_method_stanza);
                },
                Artifact::InternetPlugin {
                    internet_plugin,
                    target,
                } => {
                    let internet_plugin_stanza = CommonStanza::from((internet_plugin, target));

                    this.internet_plugin.push(internet_plugin_stanza);
                },
                Artifact::KeyboardLayout {
                    keyboard_layout,
                    target,
                } => {
                    let keyboard_layout_stanza = CommonStanza::from((keyboard_layout, target));

                    this.keyboard_layout.push(keyboard_layout_stanza);
                },
                Artifact::Prefpane {
                    prefpane,
                    target,
                } => {
                    let prefpane_stanza = CommonStanza::from((prefpane, target));

                    this.prefpane.push(prefpane_stanza);
                },
                Artifact::Mdimporter {
                    mdimporter,
                    target,
                } => {
                    let mdimporter_stanza = CommonStanza::from((mdimporter, target));

                    this.mdimporter.push(mdimporter_stanza);
                },
                Artifact::ScreenSaver {
                    screen_saver,
                    target,
                } => {
                    let screen_saver_stanza = CommonStanza::from((screen_saver, target));

                    this.screen_saver.push(screen_saver_stanza);
                },
                Artifact::Service {
                    service,
                    target,
                } => {
                    let service_stanza = CommonStanza::from((service, target));

                    this.service.push(service_stanza);
                },
                Artifact::AudioUnitPlugin {
                    audio_unit_plugin,
                    target,
                } => {
                    let audio_unit_plugin_stanza = CommonStanza::from((audio_unit_plugin, target));

                    this.audio_unit_plugin.push(audio_unit_plugin_stanza);
                },
                Artifact::VstPlugin {
                    vst_plugin,
                    target,
                } => {
                    let vst_plugin_stanza = CommonStanza::from((vst_plugin, target));

                    this.vst_plugin.push(vst_plugin_stanza);
                },
                Artifact::Vst3Plugin {
                    vst3_plugin,
                    target,
                } => {
                    let vst3_plugin_stanza = CommonStanza::from((vst3_plugin, target));

                    this.vst3_plugin.push(vst3_plugin_stanza);
                },
                Artifact::Artifact {
                    artifact,
                    target,
                } => {
                    let artifact_stanza = CommonStanza::from((artifact, target));

                    this.artifact.push(artifact_stanza);
                },
                Artifact::StageOnly {
                    stage_only,
                } => {
                    this.stage_only = stage_only.0.0;
                },
                Artifact::Unsupported(_) => {},
            }
        }

        this
    }
}

pub(crate) struct CommonStanza {
    pub(crate) source: String,
    pub(crate) target: String,
    pub(crate) rename: Option<String>,
}

impl From<(ArtifactCommonSource, String)> for CommonStanza {
    fn from((artifact, target): (ArtifactCommonSource, String)) -> Self {
        let (source, rename) = match artifact {
            ArtifactCommonSource::WithOptions(source, option) => (source, Some(option.target)),
            ArtifactCommonSource::WithoutOptions((source,)) => (source, None),
        };

        Self {
            source,
            target,
            rename,
        }
    }
}

struct PkgStanza {
    source: String,
    allow_untrusted: bool,
    choices: Vec<ArtifactPkgSourceOptionsChoice>,
}

impl From<ArtifactPkgSource> for PkgStanza {
    fn from(pkg: ArtifactPkgSource) -> Self {
        let (source, allow_untrusted, choices) = match pkg {
            ArtifactPkgSource::WithOptions(source, options) => {
                (source, options.allow_untrusted, options.choices)
            },
            ArtifactPkgSource::WithoutOptions((source,)) => (source, false, Vec::new()),
        };

        Self {
            source,
            allow_untrusted,
            choices,
        }
    }
}

struct GenerateCompletionsFromExecutableStanza {
    command: String,
    subcommand: String,
    base_name: Option<String>,
    shell_parameter_format: Option<String>,
    shells: Vec<String>,
}

impl From<ArtifactGenerateCompletionsFromExecutableSource>
    for GenerateCompletionsFromExecutableStanza
{
    fn from(
        generate_completions_from_executable: ArtifactGenerateCompletionsFromExecutableSource,
    ) -> Self {
        Self {
            command: generate_completions_from_executable.0,
            subcommand: generate_completions_from_executable.1,
            base_name: generate_completions_from_executable.2.base_name,
            shell_parameter_format: generate_completions_from_executable
                .2
                .shell_parameter_format,
            shells: generate_completions_from_executable.2.shells,
        }
    }
}
