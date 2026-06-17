use std::{path::Path, process::Stdio};

use anyhow::{Context as _, anyhow};
use futures::future;
use tempfile::NamedTempFile;
use tokio::{fs, io::AsyncWriteExt as _, process::Command};

use super::{Artifactor, ArtifactorExt, ReplacementPairs};
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    package::prepared::cask_stanza::{CommonStanza, InstallerStanza, PkgStanza, Stanzas},
};

impl ArtifactorExt for Artifactor {
    async fn install(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        self.install_installer(
            &stanzas.installer,
            staged_dir_path,
            replacement_pairs,
            context,
        )
        .await?;

        self.install_pkg(&stanzas.pkg, staged_dir_path, replacement_pairs, context)
            .await?;

        Ok(())
    }

    async fn relocate(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        self.relocate_commons(stanzas, staged_dir_path, replacement_pairs, context)
            .await?;

        Ok(())
    }

    async fn link(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        self.link_commons(stanzas, staged_dir_path, replacement_pairs, context)
            .await?;

        Ok(())
    }
}

impl Artifactor {
    async fn install_installer(
        &self,
        installer_stanzas: &[InstallerStanza],
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        if installer_stanzas.is_empty() {
            return Ok(());
        }

        for installer_stanza in installer_stanzas {
            let InstallerStanza::Script {
                script,
            } = installer_stanza
            else {
                continue;
            };

            if script.sudo {
                continue;
            }

            let script_executable_pstr = &script.executable;

            let script_executable_path =
                self.resolve_source(script_executable_pstr, replacement_pairs);

            let script_executable = if script_executable_path.is_relative() {
                let script_executable = staged_dir_path.join(&script_executable_path);

                let is_script_executable_exists =
                    script_executable.try_exists_follow().await.unwrap_or(false);

                if is_script_executable_exists {
                    script_executable.add_permissions_mode(0o111).await?;

                    script_executable
                } else {
                    script_executable_path
                }
            } else {
                script_executable_path
            };

            let mut script_cmd = Command::new(script_executable);

            script_cmd.current_dir(staged_dir_path);

            let script_args = script
                .args
                .iter()
                .map(|script_arg| self.resolve_target(script_arg, replacement_pairs, context))
                .collect::<Vec<_>>();

            script_cmd.args(script_args);

            let script_output = if script.input.is_empty() {
                script_cmd.output().await?
            } else {
                script_cmd
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                let mut script_child = script_cmd.spawn()?;

                if let Some(mut stdin) = script_child.stdin.take() {
                    for script_input in &script.input {
                        stdin.write_all(script_input.as_bytes()).await?;
                    }
                }

                script_child.wait_with_output().await?
            };

            if script.must_succeed && !script_output.status.success() {
                let stdout = String::from_utf8_lossy(&script_output.stdout);

                let stderr = String::from_utf8_lossy(&script_output.stderr);

                let err = anyhow!("{stdout}{stderr}");

                return Err(err);
            }
        }

        Ok(())
    }

    async fn install_pkg(
        &self,
        pkg_stanzas: &[PkgStanza],
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        if pkg_stanzas.is_empty() {
            return Ok(());
        }

        let dest_dir_path = context.homebrew_dirs.install_dir();

        fs::create_dir_all(&dest_dir_path).await?;

        for pkg_stanza in pkg_stanzas {
            let pkg_source_pstr = &pkg_stanza.source;

            let pkg_source_path = self.resolve_source(pkg_source_pstr, replacement_pairs);

            let src_file_path = staged_dir_path.join(pkg_source_path);

            let mut installer_cmd = Command::new("installer");

            installer_cmd.current_dir(staged_dir_path);

            installer_cmd
                .arg("-pkg")
                .arg(src_file_path)
                .arg("-target")
                .arg(&dest_dir_path);

            if pkg_stanza.allow_untrusted {
                installer_cmd.arg("-allowUntrusted");
            }

            let choices_file = if pkg_stanza.choices.is_empty() {
                None
            } else {
                let choices_file = NamedTempFile::new_in(staged_dir_path)?;

                let choices_file_path = choices_file.path();

                plist::to_file_xml(choices_file_path, &pkg_stanza.choices)?;

                installer_cmd
                    .arg("-applyChoiceChangesXML")
                    .arg(choices_file_path);

                Some(choices_file)
            };

            let installer_output = installer_cmd.output().await?;

            if let Some(choices_file) = choices_file {
                choices_file.close()?;
            }

            if !installer_output.status.success() {
                let stdout = String::from_utf8_lossy(&installer_output.stdout);

                if stdout == "installer: Must be run as root to install this package.\n" {
                    continue;
                }

                let stderr = String::from_utf8_lossy(&installer_output.stderr);

                let err = anyhow!("{stdout}{stderr}");

                return Err(err);
            }
        }

        Ok(())
    }

    async fn relocate_commons(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        let common_stanzas_list = [
            &stanzas.app,
            &stanzas.suite,
            &stanzas.colorpicker,
            &stanzas.dictionary,
            &stanzas.font,
            &stanzas.input_method,
            &stanzas.internet_plugin,
            &stanzas.keyboard_layout,
            &stanzas.prefpane,
            &stanzas.qlplugin,
            &stanzas.mdimporter,
            &stanzas.screen_saver,
            &stanzas.service,
            &stanzas.audio_unit_plugin,
            &stanzas.vst_plugin,
            &stanzas.vst3_plugin,
            &stanzas.artifact,
        ];

        let homebrew_dirs = &context.homebrew_dirs;

        let dest_dir_paths = [
            Some(homebrew_dirs.app_dir()),
            Some(homebrew_dirs.app_dir()),
            Some(homebrew_dirs.colorpicker_dir()),
            Some(homebrew_dirs.dictionary_dir()),
            Some(homebrew_dirs.font_dir()),
            Some(homebrew_dirs.input_method_dir()),
            Some(homebrew_dirs.internet_plugin_dir()),
            Some(homebrew_dirs.keyboard_layout_dir()),
            Some(homebrew_dirs.prefpane_dir()),
            Some(homebrew_dirs.qlplugin_dir()),
            Some(homebrew_dirs.mdimporter_dir()),
            Some(homebrew_dirs.screen_saver_dir()),
            Some(homebrew_dirs.service_dir()),
            Some(homebrew_dirs.audio_unit_plugin_dir()),
            Some(homebrew_dirs.vst_plugin_dir()),
            Some(homebrew_dirs.vst3_plugin_dir()),
            None,
        ];

        let common_stanzas_futs = common_stanzas_list
            .into_iter()
            .zip(dest_dir_paths.iter())
            .map(|(common_stanzas, dest_dir_path)| {
                self.relocate_common(
                    common_stanzas,
                    staged_dir_path,
                    dest_dir_path.as_deref(),
                    replacement_pairs,
                    context,
                )
            });

        future::try_join_all(common_stanzas_futs).await?;

        Ok(())
    }

    async fn relocate_common(
        &self,
        common_stanzas: &[CommonStanza],
        staged_dir_path: &Path,
        dest_dir_path: Option<&Path>,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        if common_stanzas.is_empty() {
            return Ok(());
        }

        if let Some(dest_dir_path) = dest_dir_path {
            fs::create_dir_all(dest_dir_path).await?;
        }

        let common_stanza_futs = common_stanzas.iter().map(async |common_stanza| {
            let common_source_pstr = &common_stanza.source;

            let common_source_path = self.resolve_source(common_source_pstr, replacement_pairs);

            let src_item_path = staged_dir_path.join(common_source_path);

            let common_target_pstr = &common_stanza.target;

            let common_target_path =
                self.resolve_target(common_target_pstr, replacement_pairs, context);

            let dest_item_path = if common_target_path.is_relative() {
                dest_dir_path
                    .map(|dest_dir_path| dest_dir_path.join(&common_target_path))
                    .unwrap_or(common_target_path)
            } else {
                common_target_path
            };

            let common_rename_pstr = &common_stanza.rename;

            let dest_item_path = match common_rename_pstr {
                Some(common_rename_pstr) => {
                    let common_rename_path =
                        self.resolve_target(common_rename_pstr, replacement_pairs, context);

                    if common_rename_path.is_relative() {
                        dest_item_path.with_file_name(common_rename_path)
                    } else {
                        common_rename_path
                    }
                },
                None => dest_item_path,
            };

            let dest_item_base_path = dest_item_path.base()?;

            if Some(dest_item_base_path) != dest_dir_path {
                fs::create_dir_all(dest_item_base_path).await?;
            }

            if dest_item_path.is_dir_exists_nofollow().await? {
                fs::remove_dir_all(&dest_item_path).await?;
            }

            fs::rename(&src_item_path, &dest_item_path)
                .await
                .with_context(|| {
                    let src_item_path = src_item_path.display();

                    let dest_item_path = dest_item_path.display();

                    format!(r#"Failed to rename "{src_item_path}" to "{dest_item_path}""#)
                })?;

            dest_item_path
                .create_relative_link_atomically_at(src_item_path)
                .await?;

            anyhow::Ok(())
        });

        future::try_join_all(common_stanza_futs).await?;

        Ok(())
    }

    async fn link_commons(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        let common_stanzas_list = [
            &stanzas.binary,
            &stanzas.manpage,
            &stanzas.bash_completion,
            &stanzas.fish_completion,
            &stanzas.zsh_completion,
        ];

        let homebrew_dirs = &context.homebrew_dirs;

        let dest_dir_paths = [
            homebrew_dirs.bin_dir(),
            homebrew_dirs.man_dir(),
            homebrew_dirs.bash_completion_dir(),
            homebrew_dirs.fish_completion_dir(),
            homebrew_dirs.zsh_completion_dir(),
        ];

        let permissions_modes = [Some(0o111), None, None, None, None];

        let common_stanzas_futs = common_stanzas_list
            .into_iter()
            .zip(dest_dir_paths.iter())
            .zip(permissions_modes)
            .map(|((common_stanzas, dest_dir_path), permissions_mode)| {
                self.link_common(
                    common_stanzas,
                    staged_dir_path,
                    dest_dir_path,
                    permissions_mode,
                    replacement_pairs,
                    context,
                )
            });

        future::try_join_all(common_stanzas_futs).await?;

        Ok(())
    }

    async fn link_common(
        &self,
        common_stanzas: &[CommonStanza],
        staged_dir_path: &Path,
        dest_dir_path: &Path,
        permissions_mode: Option<u32>,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        if common_stanzas.is_empty() {
            return Ok(());
        }

        fs::create_dir_all(dest_dir_path).await?;

        let common_stanza_futs = common_stanzas.iter().map(async |common_stanza| {
            let common_source_pstr = &common_stanza.source;

            let common_source_path = self.resolve_source(common_source_pstr, replacement_pairs);

            let src_item_path = staged_dir_path.join(common_source_path);

            let common_target_pstr = &common_stanza.target;

            let common_target_path =
                self.resolve_target(common_target_pstr, replacement_pairs, context);

            let dest_link_path = if common_target_path.is_relative() {
                dest_dir_path.join(common_target_path)
            } else {
                common_target_path
            };

            let common_rename_pstr = &common_stanza.rename;

            let dest_link_path = match common_rename_pstr {
                Some(common_rename_pstr) => {
                    let common_rename_path =
                        self.resolve_target(common_rename_pstr, replacement_pairs, context);

                    if common_rename_path.is_relative() {
                        dest_link_path.with_file_name(common_rename_path)
                    } else {
                        common_rename_path
                    }
                },
                None => dest_link_path,
            };

            let dest_link_base_path = dest_link_path.base()?;

            if dest_link_base_path != dest_dir_path {
                fs::create_dir_all(dest_link_base_path).await?;
            }

            src_item_path
                .create_relative_link_atomically_at(dest_link_path)
                .await?;

            if let Some(permissions_mode) = permissions_mode {
                src_item_path.add_permissions_mode(permissions_mode).await?;
            }

            anyhow::Ok(())
        });

        future::try_join_all(common_stanza_futs).await?;

        Ok(())
    }
}
