use std::path::{Path, PathBuf};

use anyhow::Context as _;
use futures::future;
use tokio::fs;

use super::{Artifactor, Artifactory, ReplacementPairs};
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    package::{
        Packageable as _,
        prepared::{CommonStanza, Download, PreparedCask, Stanzas},
    },
};

impl Artifactory for Artifactor {
    async fn relocate(
        &self,
        prepared_cask: &PreparedCask<Download>,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<PathBuf> {
        let id = prepared_cask.id();

        let version = prepared_cask.version();

        let staged_dir_path = context.homebrew_dirs.staged_dir(id, version);

        let stanzas = prepared_cask.stanzas();

        self.relocate_commons(stanzas, &staged_dir_path, replacement_pairs, context)
            .await?;

        Ok(staged_dir_path)
    }

    async fn link(
        &self,
        prepared_cask: &PreparedCask<Download>,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<PathBuf> {
        let id = prepared_cask.id();

        let version = prepared_cask.version();

        let staged_dir_path = context.homebrew_dirs.staged_dir(id, version);

        let stanzas = &prepared_cask.stanzas();

        self.link_commons(stanzas, &staged_dir_path, replacement_pairs, context)
            .await?;

        Ok(staged_dir_path)
    }
}

impl Artifactor {
    async fn relocate_commons(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        let common_stanzas = vec![
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

        let dest_dir_paths = vec![
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

        let common_stanza_futs =
            common_stanzas
                .into_iter()
                .zip(&dest_dir_paths)
                .map(|(stanzas, dest_dir_path)| {
                    self.relocate_common(
                        stanzas,
                        staged_dir_path,
                        dest_dir_path.as_deref(),
                        replacement_pairs,
                        context,
                    )
                });

        future::try_join_all(common_stanza_futs).await?;

        Ok(())
    }

    async fn relocate_common(
        &self,
        stanzas: &[CommonStanza],
        staged_dir_path: &Path,
        dest_dir_path: Option<&Path>,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        if stanzas.is_empty() {
            return Ok(());
        }

        if let Some(dest_dir_path) = dest_dir_path {
            fs::create_dir_all(dest_dir_path).await?;
        }

        let stanza_futs = stanzas.iter().map(async |stanza| {
            let stanza_source_pstr = &stanza.source;

            let stanza_source_path = self.resolve_source(stanza_source_pstr, replacement_pairs);

            let src_item_path = staged_dir_path.join(stanza_source_path);

            let stanza_target_pstr = &stanza.target;

            let stanza_target_path =
                self.resolve_target(stanza_target_pstr, replacement_pairs, context);

            let dest_item_path = if stanza_target_path.is_relative() {
                dest_dir_path
                    .map(|dest_dir_path| dest_dir_path.join(&stanza_target_path))
                    .unwrap_or(stanza_target_path)
            } else {
                stanza_target_path
            };

            let stanza_rename_pstr = &stanza.rename;

            let dest_item_path = match stanza_rename_pstr {
                Some(stanza_rename_pstr) => {
                    let stanza_rename_path =
                        self.resolve_target(stanza_rename_pstr, replacement_pairs, context);

                    if stanza_rename_path.is_relative() {
                        dest_item_path.with_file_name(stanza_rename_path)
                    } else {
                        stanza_rename_path
                    }
                },
                None => dest_item_path,
            };

            let dest_item_base_path = dest_item_path.base()?;

            fs::create_dir_all(dest_item_base_path).await?;

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

        future::try_join_all(stanza_futs).await?;

        Ok(())
    }

    async fn link_commons(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        let common_stanzas = vec![
            &stanzas.binary,
            &stanzas.manpage,
            &stanzas.bash_completion,
            &stanzas.fish_completion,
            &stanzas.zsh_completion,
        ];

        let homebrew_dirs = &context.homebrew_dirs;

        let dest_dir_paths = vec![
            homebrew_dirs.bin_dir(),
            homebrew_dirs.man_dir(),
            homebrew_dirs.bash_completion_dir(),
            homebrew_dirs.fish_completion_dir(),
            homebrew_dirs.zsh_completion_dir(),
        ];

        let permissions_modes = [Some(0o111), None, None, None, None];

        let common_stanza_futs = common_stanzas
            .into_iter()
            .zip(&dest_dir_paths)
            .zip(permissions_modes)
            .map(|((stanzas, dest_dir_path), permissions_mode)| {
                self.link_common(
                    stanzas,
                    staged_dir_path,
                    dest_dir_path,
                    permissions_mode,
                    replacement_pairs,
                    context,
                )
            });

        future::try_join_all(common_stanza_futs).await?;

        Ok(())
    }

    async fn link_common(
        &self,
        stanzas: &[CommonStanza],
        staged_dir_path: &Path,
        dest_dir_path: &Path,
        permissions_mode: Option<u32>,
        replacement_pairs: &ReplacementPairs,
        context: &Context,
    ) -> anyhow::Result<()> {
        if stanzas.is_empty() {
            return Ok(());
        }

        fs::create_dir_all(dest_dir_path).await?;

        let stanza_futs = stanzas.iter().map(async |stanza| {
            let stanza_source_pstr = &stanza.source;

            let stanza_source_path = self.resolve_source(stanza_source_pstr, replacement_pairs);

            let src_item_path = staged_dir_path.join(stanza_source_path);

            let stanza_target_pstr = &stanza.target;

            let stanza_target_path =
                self.resolve_target(stanza_target_pstr, replacement_pairs, context);

            let dest_link_path = if stanza_target_path.is_relative() {
                dest_dir_path.join(stanza_target_path)
            } else {
                stanza_target_path
            };

            let stanza_rename_pstr = &stanza.rename;

            let dest_link_path = match stanza_rename_pstr {
                Some(stanza_rename_pstr) => {
                    let stanza_rename_path =
                        self.resolve_target(stanza_rename_pstr, replacement_pairs, context);

                    if stanza_rename_path.is_relative() {
                        dest_link_path.with_file_name(stanza_rename_path)
                    } else {
                        stanza_rename_path
                    }
                },
                None => dest_link_path,
            };

            let dest_link_base_path = dest_link_path.base()?;

            fs::create_dir_all(dest_link_base_path).await?;

            src_item_path
                .create_relative_link_atomically_at(dest_link_path)
                .await?;

            if let Some(permissions_mode) = permissions_mode {
                src_item_path.add_permissions_mode(permissions_mode).await?;
            }

            anyhow::Ok(())
        });

        future::try_join_all(stanza_futs).await?;

        Ok(())
    }
}
