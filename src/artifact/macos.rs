use std::{path::Path, sync::Arc};

use anyhow::Context as _;
use futures::future;
use tokio::fs;

use super::Artifactable;
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    package::{
        Packageable as _,
        prepared::{CommonStanza, PreparedCask, Stanzas},
    },
    placeholder::Placeholder,
};

pub(crate) struct Artifact {
    placeholder: Arc<Placeholder>,

    context: Arc<Context>,
}

impl Artifactable for Artifact {
    fn new(placeholder: Arc<Placeholder>, context: Arc<Context>) -> Self {
        Self {
            placeholder,

            context,
        }
    }

    async fn relocate(&self, prepared_cask: &PreparedCask) -> anyhow::Result<()> {
        let id = prepared_cask.id();

        let version = prepared_cask.version();

        let staged_dir_path = self.context.homebrew_dirs.staged_dir(id, version);

        let stanzas = prepared_cask.stanzas();

        self.relocate_commons(stanzas, &staged_dir_path).await?;

        Ok(())
    }
}

impl Artifact {
    async fn relocate_commons(
        &self,
        stanzas: &Stanzas,
        staged_dir_path: &Path,
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

        let homebrew_dirs = &self.context.homebrew_dirs;

        let dest_base_paths = vec![
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
                .zip(&dest_base_paths)
                .map(|(stanzas, dest_base_path)| {
                    self.relocate_common(stanzas, staged_dir_path, dest_base_path.as_deref())
                });

        future::try_join_all(common_stanza_futs).await?;

        Ok(())
    }

    async fn relocate_common(
        &self,
        stanzas: &[CommonStanza],
        staged_dir_path: &Path,
        dest_base_path: Option<&Path>,
    ) -> anyhow::Result<()> {
        if stanzas.is_empty() {
            return Ok(());
        }

        if let Some(dest_base_path) = dest_base_path {
            fs::create_dir_all(dest_base_path).await?;
        }

        let stanza_futs = stanzas.iter().map(async |stanza| {
            let stanza_source_pstr = &stanza.source;

            let stanza_source_path = self.placeholder.resolve_source(stanza_source_pstr);

            let src_item_path = staged_dir_path.join(stanza_source_path);

            let stanza_target_pstr = &stanza.target;

            let stanza_target_path = self.placeholder.resolve_target(stanza_target_pstr);

            let dest_item_path = if stanza_target_path.is_relative() {
                dest_base_path
                    .map(|dest_base_path| dest_base_path.join(&stanza_target_path))
                    .unwrap_or(stanza_target_path)
            } else {
                stanza_target_path
            };

            let stanza_rename_pstr = &stanza.rename;

            let dest_item_path = match stanza_rename_pstr {
                Some(stanza_rename_pstr) => {
                    let stanza_rename_path = self.placeholder.resolve_target(stanza_rename_pstr);

                    if stanza_rename_path.is_relative() {
                        dest_item_path.with_file_name(stanza_rename_path)
                    } else {
                        stanza_rename_path
                    }
                },
                None => dest_item_path,
            };

            let dest_base_path = dest_item_path.base()?;

            fs::create_dir_all(dest_base_path).await?;

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
}
