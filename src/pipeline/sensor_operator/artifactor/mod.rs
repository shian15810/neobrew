#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, anyhow};
use async_trait::async_trait;
use futures::future;
use path_clean::PathClean as _;
use tokio::{fs, process::Command};

use super::{
    super::state_store::{ArtifactedOutput, ExtractedOutput, Stage},
    SensorOperator,
};
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    package::{
        PackageExt as _,
        prepared::{
            PreparedPackage,
            cask_stanza::{
                CompletionsStanza,
                CompletionsStanzaFormat,
                CompletionsStanzaShell,
                Stanzas,
            },
            download::Download,
        },
    },
};

#[cfg(target_os = "macos")]
type ReplacementPairs = [(&'static str, String); 4];

#[cfg(target_os = "linux")]
type ReplacementPairs = [(&'static str, String); 3];

pub(crate) struct Artifactor;

#[async_trait]
impl SensorOperator for Artifactor {
    type Payload = ExtractedOutput;
    type State = ReplacementPairs;
    type Staging = PathBuf;
    type Output = ArtifactedOutput;

    fn poke_stage(&self) -> Stage {
        Stage::Extracted
    }

    fn should_run(
        &self,
        payload: Option<&Self::Payload>,
        prepared_package: &PreparedPackage<Download>,
        _context: &Context,
    ) -> bool {
        let Some(_payload) = payload else {
            return false;
        };

        let PreparedPackage::Cask(_prepared_cask) = prepared_package else {
            return false;
        };

        true
    }

    fn running_prefix(&self) -> Option<&'static str> {
        Some("Installing")
    }

    fn init(&self, context: &Context) -> anyhow::Result<Self::State> {
        let homebrew_dirs = &context.homebrew_dirs;

        #[cfg(target_os = "macos")]
        let replacement_pairs = [
            ("/$HOME", homebrew_dirs.home_dir()),
            ("$HOMEBREW_PREFIX", homebrew_dirs.prefix_dir()),
            ("$HOMEBREW_CELLAR", homebrew_dirs.cellar_dir()),
            ("$APPDIR", homebrew_dirs.app_dir()),
        ];

        #[cfg(target_os = "linux")]
        let replacement_pairs = [
            ("/$HOME", homebrew_dirs.home_dir()),
            ("$HOMEBREW_PREFIX", homebrew_dirs.prefix_dir()),
            ("$HOMEBREW_CELLAR", homebrew_dirs.cellar_dir()),
        ];

        let replacement_pairs = replacement_pairs.map(|(placeholder, replacement_path)| {
            let replacement_pstr = replacement_path.to_string_lossy();
            let replacement_pstr = replacement_pstr.into_owned();

            (placeholder, replacement_pstr)
        });

        let state = replacement_pairs;

        Ok(state)
    }

    async fn execute(
        &self,
        state: &Self::State,
        prepared_package: &PreparedPackage<Download>,
        context: &Context,
    ) -> anyhow::Result<Self::Staging> {
        let PreparedPackage::Cask(prepared_cask) = prepared_package else {
            let err = anyhow!("`PreparedFormula` is not supposed to be artifacted");

            return Err(err);
        };

        let replacement_pairs = state;

        let stanzas = prepared_cask.stanzas();

        let id = prepared_cask.id();

        let version = prepared_cask.version();

        let staged_dir_path = context.homebrew_dirs.staged_dir(id, version);

        if stanzas.stage_only {
            let staging = staged_dir_path;

            return Ok(staging);
        }

        self.install(stanzas, &staged_dir_path, replacement_pairs, context)
            .await?;

        self.relocate(stanzas, &staged_dir_path, replacement_pairs, context)
            .await?;

        self.link(stanzas, &staged_dir_path, replacement_pairs, context)
            .await?;

        self.generate_completions(
            &stanzas.completions,
            &staged_dir_path,
            replacement_pairs,
            context,
        )
        .await?;

        let staging = staged_dir_path;

        Ok(staging)
    }

    fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output> {
        let staged_dir_path = staging;

        let output = ArtifactedOutput {
            staged_dir_path,
        };

        Ok(output)
    }

    fn passed_stage(
        &self,
        _should_run: bool,
        prepared_package: &PreparedPackage<Download>,
    ) -> Option<Stage> {
        let PreparedPackage::Cask(_prepared_cask) = prepared_package else {
            return None;
        };

        Some(Stage::Artifacted)
    }
}

impl Artifactor {
    async fn generate_completions(
        &self,
        completions_stanzas: &[CompletionsStanza],
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        let completions_stanza_futs = completions_stanzas.iter().map(async |completions_stanza| {
            let completions_command_pstr = &completions_stanza.command;

            let completions_command_path =
                self.resolve_source(completions_command_pstr, replacement_pairs);

            let completions_command = if completions_command_path.is_relative() {
                let completions_command = staged_dir_path.join(&completions_command_path);

                let is_completions_command_exists = completions_command
                    .try_exists_follow()
                    .await
                    .unwrap_or(false);

                if is_completions_command_exists {
                    completions_command.add_permissions_mode(0o111).await?;

                    completions_command
                } else {
                    completions_command_path
                }
            } else {
                completions_command_path
            };

            let completions_name = completions_stanza.name.clone();
            #[expect(clippy::let_and_return)]
            let completions_name = completions_name
                .or_else(|| {
                    let command_file_name = completions_command.file_name();

                    command_file_name.map(|command_file_name| {
                        let command_file_name = command_file_name.to_string_lossy();
                        let command_file_name = command_file_name.into_owned();

                        command_file_name
                    })
                })
                .context("Command name to generate completions for is missing")?;

            let completion_stanza_futs =
                completions_stanza
                    .shells
                    .iter()
                    .copied()
                    .map(async |completion_shell| {
                        let mut completion_cmd = Command::new(&completions_command);

                        completion_cmd.current_dir(staged_dir_path);

                        let mut completion_envs = HashMap::new();

                        completion_envs.insert("SHELL".to_owned(), completion_shell.to_string());

                        if let Some(completion_subcommand) = &completions_stanza.subcommand {
                            completion_cmd.arg(completion_subcommand);
                        }

                        let completion_args = self.generate_completions_args(
                            &completions_command,
                            completions_stanza.format.as_ref(),
                            completion_shell,
                            &mut completion_envs,
                        );

                        completion_cmd.envs(completion_envs);

                        if let Some(completion_args) = completion_args {
                            completion_cmd.args(completion_args);
                        }

                        let completion_output = completion_cmd.output().await?;

                        if !completion_output.status.success() {
                            let stdout = String::from_utf8_lossy(&completion_output.stdout);

                            let stderr = String::from_utf8_lossy(&completion_output.stderr);

                            let err = anyhow!("{stdout}{stderr}");

                            return Err(err);
                        }

                        let completion_file_path = match completion_shell {
                            CompletionsStanzaShell::Bash => context
                                .homebrew_dirs
                                .bash_completion_file(&completions_name),
                            CompletionsStanzaShell::Fish => context
                                .homebrew_dirs
                                .fish_completion_file(&completions_name),
                            CompletionsStanzaShell::Zsh => {
                                context.homebrew_dirs.zsh_completion_file(&completions_name)
                            },
                            CompletionsStanzaShell::Pwsh => context
                                .homebrew_dirs
                                .pwsh_completion_file(&completions_name),
                        };

                        let completion_file_base_path = completion_file_path.base()?;

                        fs::create_dir_all(completion_file_base_path).await?;

                        fs::write(completion_file_path, completion_output.stdout).await?;

                        anyhow::Ok(())
                    });

            future::try_join_all(completion_stanza_futs).await?;

            anyhow::Ok(())
        });

        future::try_join_all(completions_stanza_futs).await?;

        Ok(())
    }

    #[expect(clippy::unused_self)]
    fn generate_completions_args(
        &self,
        command: &Path,
        format: Option<&CompletionsStanzaFormat>,
        shell: CompletionsStanzaShell,
        envs: &mut HashMap<String, String>,
    ) -> Option<Vec<String>> {
        let shell_parameter = if shell == CompletionsStanzaShell::Pwsh {
            "powershell"
        } else {
            shell.as_ref()
        };

        match format {
            Some(CompletionsStanzaFormat::Arg) => {
                let completions_args = vec![format!("--shell={shell_parameter}")];

                Some(completions_args)
            },
            Some(CompletionsStanzaFormat::Clap) => {
                envs.insert("COMPLETE".to_owned(), shell_parameter.to_owned());

                None
            },
            Some(CompletionsStanzaFormat::Click) => {
                let command_file_name = command.file_name()?;
                let command_file_name = command_file_name.to_string_lossy();
                let command_file_name = command_file_name.to_uppercase().replace('-', "_");

                envs.insert(
                    format!("_{command_file_name}_COMPLETE"),
                    format!("{shell_parameter}_source"),
                );

                None
            },
            Some(CompletionsStanzaFormat::Cobra) => {
                let completions_args = vec!["completion".to_owned(), shell_parameter.to_owned()];

                Some(completions_args)
            },
            Some(CompletionsStanzaFormat::Flag) => {
                let completions_args = vec![format!("--{shell_parameter}")];

                Some(completions_args)
            },
            Some(CompletionsStanzaFormat::None) => None,
            Some(CompletionsStanzaFormat::Typer) => {
                envs.insert(
                    "_TYPER_COMPLETE_TEST_DISABLE_SHELL_DETECTION".to_owned(),
                    "1".to_owned(),
                );

                let completions_args =
                    vec!["--show-completion".to_owned(), shell_parameter.to_owned()];

                Some(completions_args)
            },
            Some(CompletionsStanzaFormat::Other(format)) => {
                let completions_args = vec![format!("{format}{shell}")];

                Some(completions_args)
            },
            None => {
                let completions_args = vec![shell_parameter.to_owned()];

                Some(completions_args)
            },
        }
    }

    fn resolve_source(&self, pstr: &str, replacement_pairs: &ReplacementPairs) -> PathBuf {
        self.replace_pstr(pstr, replacement_pairs)
    }

    #[cfg(debug_assertions)]
    fn resolve_target(
        &self,
        pstr: &str,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> PathBuf {
        let path = self.replace_pstr(pstr, replacement_pairs);

        if path.is_relative() {
            return path;
        }

        let data_dir_path = context.homebrew_dirs.data_dir();

        let prefix_dir_path = context.homebrew_dirs.prefix_dir();

        if path.starts_with(&data_dir_path) || path.starts_with(prefix_dir_path) {
            return path;
        }

        match path.strip_prefix("/") {
            Ok(suffix_path) => data_dir_path.join(suffix_path),
            Err(_) => data_dir_path.join(path),
        }
    }

    #[cfg(not(debug_assertions))]
    fn resolve_target(
        &self,
        pstr: &str,
        replacement_pairs: &ReplacementPairs,
        _context: &Context,
    ) -> PathBuf {
        self.replace_pstr(pstr, replacement_pairs)
    }

    #[expect(clippy::unused_self)]
    fn replace_pstr(&self, pstr: &str, replacement_pairs: &ReplacementPairs) -> PathBuf {
        let pstr = pstr.to_owned();

        let pstr = match pstr.strip_prefix("~/") {
            Some(suffix_pstr) => format!("/$HOME/{suffix_pstr}"),
            None if pstr == "~" => "/$HOME".to_owned(),
            None => pstr,
        };

        #[cfg(target_os = "macos")]
        let pstr = match pstr.strip_prefix("/Applications/") {
            Some(suffix_pstr) => format!("$APPDIR/{suffix_pstr}"),
            None if pstr == "/Applications" => "$APPDIR".to_owned(),
            None => pstr,
        };

        let pstr = replacement_pairs
            .iter()
            .fold(pstr, |pstr, (placeholder, replacement_pstr)| {
                pstr.replace(placeholder, replacement_pstr)
            });

        let path = PathBuf::from(pstr);

        path.clean()
    }
}

trait ArtifactorExt {
    async fn install(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()>;

    async fn relocate(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()>;

    async fn link(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()>;
}
