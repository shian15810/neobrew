use std::{os::unix::fs::PermissionsExt as _, path::Path, sync::Arc};

use futures::future;
use tokio::fs;

use super::Linkerer;
use crate::{
    context::Context,
    ext::{std::path::PathExt as _, tokio::path::PathExt as _},
    package::{
        Packageable as _,
        prepared::{CommonStanza, PreparedCask, Stanzas},
        streamed::StreamedCask,
    },
    placeholder::Placeholder,
};

const MUST_EXIST_DIR_NAMES: &[&str] = &["Caskroom"];

pub(super) struct CaskLinker {
    placeholder: Arc<Placeholder>,

    context: Arc<Context>,
}

impl Linkerer for CaskLinker {
    type PreparedPackage = PreparedCask;
    type StreamedPackage = StreamedCask;

    async fn is_installed(&self, prepared_package: &PreparedCask) -> anyhow::Result<bool> {
        let prepared_cask = prepared_package;

        let id = prepared_cask.id();

        let cask_dir_path = self.context.homebrew_dirs.cask_dir(id);

        if !cask_dir_path.is_dir_exists_nofollow().await? {
            return Ok(false);
        }

        let mut cask_dir_entries = fs::read_dir(cask_dir_path).await?;

        while let Some(cask_dir_entry) = cask_dir_entries.next_entry().await? {
            let cask_dir_entry_path = cask_dir_entry.path();

            let is_cask_dir_entry_exists = cask_dir_entry_path.is_dir_exists_nofollow().await?;

            let is_cask_dir_entry_not_empty = !cask_dir_entry_path.is_dir_empty().await?;

            if is_cask_dir_entry_exists && is_cask_dir_entry_not_empty {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn is_up_to_date(&self, prepared_package: &PreparedCask) -> anyhow::Result<bool> {
        let prepared_cask = prepared_package;

        let id = prepared_cask.id();

        let version = prepared_cask.version();

        let staged_dir_path = self.context.homebrew_dirs.staged_dir(id, version);

        let is_staged_dir_exists = staged_dir_path.is_dir_exists_nofollow().await?;

        let is_staged_dir_not_empty = !staged_dir_path.is_dir_empty().await?;

        if is_staged_dir_exists && is_staged_dir_not_empty {
            return Ok(true);
        }

        Ok(false)
    }

    async fn link(&self, streamed_package: &StreamedCask) -> anyhow::Result<()> {
        let streamed_cask = streamed_package;

        let id = streamed_cask.id();

        let version = streamed_cask.version();

        let staged_dir_path = self.context.homebrew_dirs.staged_dir(id, version);

        let stanzas = &streamed_cask.stanzas();

        self.link_commons(stanzas, &staged_dir_path).await?;

        Ok(())
    }
}

impl CaskLinker {
    pub(super) async fn try_init(
        placeholder: Arc<Placeholder>,
        context: Arc<Context>,
    ) -> anyhow::Result<Self> {
        let prefix_dir_path = context.homebrew_dirs.prefix_dir();

        for must_exist_dir_name in MUST_EXIST_DIR_NAMES {
            let must_exist_dir_path = prefix_dir_path.join(must_exist_dir_name);

            fs::create_dir_all(must_exist_dir_path).await?;
        }

        let this = Self {
            placeholder,

            context,
        };

        Ok(this)
    }

    async fn link_commons(&self, stanzas: &Stanzas, staged_dir_path: &Path) -> anyhow::Result<()> {
        let common_stanzass = vec![
            &stanzas.binary,
            &stanzas.manpage,
            &stanzas.bash_completion,
            &stanzas.fish_completion,
            &stanzas.zsh_completion,
        ];

        let homebrew_dirs = &self.context.homebrew_dirs;

        let dest_base_paths = vec![
            homebrew_dirs.bin_dir(),
            homebrew_dirs.man_dir(),
            homebrew_dirs.bash_completion_dir(),
            homebrew_dirs.fish_completion_dir(),
            homebrew_dirs.zsh_completion_dir(),
        ];

        let permissions_modes = [Some(0o111), None, None, None, None];

        let common_stanzas_futs = common_stanzass
            .into_iter()
            .zip(&dest_base_paths)
            .zip(permissions_modes)
            .map(|((stanzas, dest_base_path), permissions_mode)| {
                self.link_common(stanzas, staged_dir_path, dest_base_path, permissions_mode)
            });

        future::try_join_all(common_stanzas_futs).await?;

        Ok(())
    }

    async fn link_common(
        &self,
        stanzas: &[CommonStanza],
        staged_dir_path: &Path,
        dest_base_path: &Path,
        permissions_mode: Option<u32>,
    ) -> anyhow::Result<()> {
        if stanzas.is_empty() {
            return Ok(());
        }

        fs::create_dir_all(dest_base_path).await?;

        for stanza in stanzas {
            let stanza_source_pstr = &stanza.source;

            let stanza_source_path = self.placeholder.resolve_source(stanza_source_pstr);

            let src_path = staged_dir_path.join(stanza_source_path);

            let stanza_target_pstr = &stanza.target;

            let stanza_target_path = self.placeholder.resolve_target(stanza_target_pstr);

            let dest_path = if stanza_target_path.is_relative() {
                dest_base_path.join(stanza_target_path)
            } else {
                stanza_target_path
            };

            let stanza_rename_pstr = &stanza.rename;

            let dest_path = match stanza_rename_pstr {
                Some(stanza_rename_pstr) => {
                    let stanza_rename_path = self.placeholder.resolve_target(stanza_rename_pstr);

                    if stanza_rename_path.is_relative() {
                        dest_path.with_file_name(stanza_rename_path)
                    } else {
                        stanza_rename_path
                    }
                },
                None => dest_path,
            };

            let dest_base_path = dest_path.base()?;

            fs::create_dir_all(dest_base_path).await?;

            src_path
                .create_relative_symlink_atomically_at(dest_path)
                .await?;

            if let Some(permissions_mode) = permissions_mode {
                let mut permissions = fs::symlink_metadata(&src_path).await?.permissions();

                permissions.set_mode(permissions.mode() | permissions_mode);

                fs::set_permissions(src_path, permissions).await?;
            }
        }

        Ok(())
    }
}
