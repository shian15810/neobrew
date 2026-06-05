use std::sync::Arc;

use clap::Args;
use frunk::hlist_pat;
use futures::stream::StreamExt as _;
use indicatif::{MultiProgress, ProgressBar};
use tokio::task::JoinSet;

use super::Runner;
use crate::{
    artifact::{Artifact, Artifactable as _},
    compatibility::{Compatibility, Compatibilizer as _},
    context::Context,
    downloads::Downloads,
    ext::core::result::ResultExt as _,
    linkers::Linkers,
    package::{
        Packageable as _,
        pipelined::PipelinedPackage,
        prepared::{PreparedPackage, PreparedPackageable as _},
        resolved::ResolvedPackage,
    },
    pipeline::{
        Connector as _,
        Pipeline,
        action_operator::DmgPourer,
        pull_connector::Pourer,
        push_connector::{Hasher, Progressor, Writer},
    },
    placeholder::Placeholder,
    registries::Registries,
    relocation::{Relocation, Relocator as _},
    streams::Streams,
};

#[derive(Args)]
pub(super) struct Install {
    #[arg(value_name = "PACKAGE")]
    packages: Vec<String>,
}

impl Runner for Install {
    async fn run_parallelly(self, context: Arc<Context>) -> anyhow::Result<()> {
        let installation = Installation::prepare(self.packages, context).await?;

        installation.start().await?;

        Ok(())
    }
}

struct Installation {
    packages: Vec<String>,

    multi_pb: MultiProgress,

    compatibility: Compatibility,

    downloads: Downloads,
    streams: Streams,

    relocation: Relocation,
    artifact: Artifact,
    linkers: Linkers,

    placeholder: Arc<Placeholder>,

    context: Arc<Context>,
}

impl Installation {
    async fn prepare(packages: Vec<String>, context: Arc<Context>) -> anyhow::Result<Arc<Self>> {
        let placeholder = Placeholder::new(Arc::clone(&context));
        let placeholder = Arc::new(placeholder);

        let this = Self {
            packages,

            multi_pb: MultiProgress::new(),

            compatibility: Compatibility::try_new()?,

            downloads: Downloads::new(Arc::clone(&context)),
            streams: Streams::new(Arc::clone(&context)),

            relocation: Relocation::new(Arc::clone(&context)),
            artifact: Artifact::new(Arc::clone(&placeholder), Arc::clone(&context)),
            linkers: Linkers::try_init(Arc::clone(&placeholder), Arc::clone(&context)).await?,

            placeholder,

            context,
        };
        let this = Arc::new(this);

        Ok(this)
    }

    async fn start(self: Arc<Self>) -> anyhow::Result<()> {
        if self.packages.is_empty() {
            return Ok(());
        }

        self.run_many().await?;

        Ok(())
    }

    async fn run_many(self: Arc<Self>) -> anyhow::Result<()> {
        let registries = Registries::new(Arc::clone(&self.context));

        let (resolved_packages, requested_package_ids) = registries.resolve(&self.packages).await?;

        let max_id_length = resolved_packages
            .iter()
            .map(|resolved_package| resolved_package.id().len())
            .max();

        let max_version_length = resolved_packages
            .iter()
            .map(|resolved_package| resolved_package.version().len())
            .max();

        #[cfg(debug_assertions)]
        let pbs = resolved_packages
            .iter()
            .map(|resolved_package| {
                let pb = Progressor::create(
                    &self.multi_pb,
                    resolved_package.id(),
                    resolved_package.version(),
                    max_id_length,
                    max_version_length,
                )?;

                pb.set_prefix("Resolving");

                anyhow::Ok(pb)
            })
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let pbs = resolved_packages
            .iter()
            .map(|resolved_package| {
                let pb = Progressor::create(
                    &self.multi_pb,
                    resolved_package.id(),
                    resolved_package.version(),
                    max_id_length,
                    max_version_length,
                )?;

                pb.set_prefix("Resolving");

                anyhow::Ok(pb)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let (resolved_packages_pbs, resolved_packages_incompatible_pbs) = resolved_packages
            .into_iter()
            .zip(pbs)
            .partition::<Vec<_>, _>(|(resolved_package, _)| match resolved_package {
                ResolvedPackage::Formula(_) => true,
                ResolvedPackage::Cask(resolved_cask) => {
                    self.compatibility.check(resolved_cask.depends_on())
                },
            });

        let (_, incompatible_pbs) = resolved_packages_incompatible_pbs
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();

        for pb in incompatible_pbs {
            pb.set_prefix("Incompatible");

            pb.finish();
        }

        let (resolved_packages, pbs) = resolved_packages_pbs
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();

        let is_requesteds = resolved_packages
            .iter()
            .map(|resolved_package| requested_package_ids.contains(resolved_package.id()))
            .collect::<Vec<_>>();

        #[cfg(debug_assertions)]
        let prepared_packages = resolved_packages
            .into_iter()
            .zip(is_requesteds)
            .map(PreparedPackage::try_from)
            .filter_map(Result::transpose_err)
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let prepared_packages = resolved_packages
            .into_iter()
            .zip(is_requesteds)
            .map(PreparedPackage::try_from)
            .filter_map(Result::transpose_err)
            .collect::<anyhow::Result<Vec<_>>>()?;

        let mut set = JoinSet::new();

        for (prepared_package, pb) in prepared_packages.into_iter().zip(pbs) {
            while set.len() >= self.context.concurrency_limit {
                if let Some(res) = set.join_next().await {
                    res??;
                }
            }

            let this = Arc::clone(&self);

            set.spawn({
                async move {
                    this.run_one(prepared_package, pb).await?;

                    anyhow::Ok(())
                }
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }

    async fn run_one(
        &self,
        prepared_package: PreparedPackage,
        pb: ProgressBar,
    ) -> anyhow::Result<()> {
        let is_installed = self.linkers.is_installed(&prepared_package).await?;

        let is_up_to_date = self.linkers.is_up_to_date(&prepared_package).await?;

        if is_installed && is_up_to_date {
            pb.set_prefix("Up-to-date");

            pb.finish();

            return Ok(());
        }

        pb.set_prefix("Preparing");

        let id = prepared_package.id();

        let version = prepared_package.version();

        let expected_sha256 = prepared_package.expected_sha256();

        let download = self
            .downloads
            .retrieve(&prepared_package, expected_sha256)
            .await?;

        let is_verified = download.is_verified;

        let write_file_path = download.file_path;

        let write_link_path = download.link_path;

        let archive_format = download.archive_format;

        let pour_dir_path = match prepared_package {
            PreparedPackage::Formula(_) => self.context.homebrew_dirs.cellar_dir(),
            PreparedPackage::Cask(_) => self.context.homebrew_dirs.staged_dir(id, version),
        };

        let (stream, content_length) = if is_verified {
            let (stream, content_length) = self.streams.download(&write_file_path).await?;

            let stream = stream.left_stream();

            (stream, Some(content_length))
        } else {
            let (stream, content_length) = self.streams.oci_or_url(&prepared_package).await?;

            let stream = stream.right_stream();

            (stream, content_length)
        };

        let hasher = Hasher::new(expected_sha256.to_owned(), !is_verified);

        let writer = Writer::try_init(write_file_path, write_link_path, !is_verified).await?;

        let pourer = Pourer::try_init(pour_dir_path.clone(), archive_format).await?;

        let dmg_pourer = DmgPourer::try_init(pour_dir_path, archive_format).await?;

        let hlist_pat![
            _progressor_output,
            _hasher_output,
            hlist_pat![_writer_output, _dmg_pourer_output],
            _pourer_output
        ] = Pipeline::build(stream, pb.clone(), Arc::clone(&self.context))
            .with_progressor(content_length)?
            .fanout(hasher)
            .fanout(writer.fanout(dmg_pourer))
            .fanout(pourer)
            .run_concurrently()
            .await?;

        let pipelined_package = PipelinedPackage::from(prepared_package);

        self.relocate(&pipelined_package, &pb).await?;

        self.link(&pipelined_package, &pb).await?;

        if is_installed && !is_up_to_date {
            pb.set_prefix("Upgraded");
        } else {
            pb.set_prefix("Installed");
        }

        pb.finish();

        Ok(())
    }

    async fn relocate(
        &self,
        pipelined_package: &PipelinedPackage,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        pb.set_prefix("Relocating");

        match pipelined_package {
            PipelinedPackage::Formula(pipelined_formula) => {
                self.relocation.patch(pipelined_formula).await?;
            },
            PipelinedPackage::Cask(pipelined_cask) => {
                self.artifact.relocate(pipelined_cask).await?;
            },
        }

        Ok(())
    }

    async fn link(
        &self,
        pipelined_package: &PipelinedPackage,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        pb.set_prefix("Linking");

        self.linkers.link(pipelined_package).await?;

        Ok(())
    }
}
