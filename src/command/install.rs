use std::sync::Arc;

use anyhow::anyhow;
use bytes::Bytes;
use clap::Args;
use frunk::hlist_pat;
use futures::stream;
use indicatif::{MultiProgress, ProgressBar};
use indoc::formatdoc;
use tokio::task::JoinSet;

use super::{Resolution, Runner};
use crate::{
    artifact::Artifact,
    compatibility::{Compatibility, Compatibilizer as _},
    context::Context,
    downloads::Downloads,
    ext::{core::result::ResultExt as _, tokio::path::PathExt as _},
    linkers::Linkers,
    package::{
        Packageable as _,
        prepared::{PreparedPackage, PreparedPackageable as _},
        resolved::ResolvedPackage,
        streamed::StreamedPackage,
    },
    pipeline::{Pipeline, handler::AtomicWriter as _, pull_operator, push_operator},
    placeholder::Placeholder,
    registries::{Registries, ResolutionStrategy},
    relocation::{Relocation, Relocator as _},
    streams::Streams,
};

#[derive(Args)]
pub(super) struct Install {
    #[command(flatten)]
    resolution: Resolution,

    #[arg(value_name = "PACKAGE")]
    packages: Vec<String>,
}

impl Runner for Install {
    async fn run_concurrent(self, context: Arc<Context>) -> anyhow::Result<()> {
        let installation = Installation::prepare(self.packages, self.resolution, context).await?;

        installation.start().await?;

        Ok(())
    }
}

struct Installation {
    packages: Vec<String>,
    resolution: Resolution,

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
    async fn prepare(
        packages: Vec<String>,
        resolution: Resolution,
        context: Arc<Context>,
    ) -> anyhow::Result<Arc<Self>> {
        let placeholder = Placeholder::new(Arc::clone(&context));
        let placeholder = Arc::new(placeholder);

        let this = Self {
            packages,
            resolution,

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

        let this = Arc::clone(&self);

        let strategy = self.resolution.strategy();

        this.run_many(&self.packages, strategy).await?;

        Ok(())
    }

    async fn run_many(
        self: Arc<Self>,
        packages: &[String],
        strategy: ResolutionStrategy,
    ) -> anyhow::Result<()> {
        let registries = Registries::new(Arc::clone(&self.context));

        let resolved_packages = registries
            .resolve(packages.iter().cloned(), strategy)
            .await?;

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
                let pb = push_operator::PbUpdater::create(
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
                let pb = push_operator::PbUpdater::create(
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

        let (resolved_packages_pbs, incompatible_resolved_packages_pbs) = resolved_packages
            .into_iter()
            .zip(pbs)
            .partition::<Vec<_>, _>(|(resolved_package, _)| match resolved_package {
                ResolvedPackage::Formula(_) => true,
                ResolvedPackage::Cask(resolved_cask) => {
                    self.compatibility.check(&resolved_cask.depends_on)
                },
            });

        let (_, incompatible_pbs) = incompatible_resolved_packages_pbs
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();

        for incompatible_pb in incompatible_pbs {
            incompatible_pb.set_prefix("Incompatible");

            incompatible_pb.finish();
        }

        let (resolved_packages, pbs) = resolved_packages_pbs
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();

        #[cfg(debug_assertions)]
        let prepared_packages = resolved_packages
            .into_iter()
            .map(PreparedPackage::try_from)
            .filter_map(Result::transpose_err)
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let prepared_packages = resolved_packages
            .into_iter()
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

            set.spawn(async move {
                this.run_one(prepared_package, pb).await?;

                anyhow::Ok(())
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
        pb.set_prefix("Preparing");

        let id = prepared_package.id();

        let version = prepared_package.version();

        let expected_sha256 = prepared_package.expected_sha256();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        if keg_dir_path.is_dir_exists_nofollow().await? {
            let streamed_package = StreamedPackage::from(prepared_package);

            self.relocate(&streamed_package, &pb).await?;

            self.link(&streamed_package, &pb).await?;

            pb.set_prefix("Installed");

            pb.finish();

            return Ok(());
        }

        let download = self
            .downloads
            .retrieve(&prepared_package, expected_sha256)
            .await?;

        let pourer_dir_path = match prepared_package {
            PreparedPackage::Formula(_) => self.context.homebrew_dirs.cellar_dir(),
            PreparedPackage::Cask(_) => self.context.homebrew_dirs.staged_dir(id, version),
        };

        let temp_pourer =
            pull_operator::TempPourer::new(download.archive_format, pourer_dir_path, vec![]);

        let sha256_hasher = push_operator::Sha256Hasher::new();

        let pb = if download.is_valid {
            let (stream, content_length) = self.streams.download(&download.file_path).await?;

            let content_length = Some(content_length);

            let pb_updater = push_operator::PbUpdater::try_new(pb, content_length)?;

            let operators = Operators {
                temp_pourer,
                temp_writer: (),
                sha256_hasher,
                pb_updater,
            };

            self.stream_from_download(id, version, expected_sha256, operators, stream)
                .await?
        } else {
            let temp_writer =
                push_operator::TempWriter::new(download.file_path, vec![download.symlink_path])
                    .await?;

            let (stream, content_length) = self.streams.oci_or_url(&prepared_package).await?;

            let pb_updater = push_operator::PbUpdater::try_new(pb, content_length)?;

            let operators = Operators {
                pb_updater,
                sha256_hasher,
                temp_writer,
                temp_pourer,
            };

            self.stream_from_oci_or_url(id, version, expected_sha256, operators, stream)
                .await?
        };

        let streamed_package = StreamedPackage::from(prepared_package);

        self.relocate(&streamed_package, &pb).await?;

        self.link(&streamed_package, &pb).await?;

        pb.set_prefix("Installed");

        pb.finish();

        Ok(())
    }

    async fn stream_from_download(
        &self,
        id: &str,
        version: &str,
        expected_sha256: &str,
        operators: Operators<()>,
        stream: impl stream::Stream<Item = anyhow::Result<Bytes>> + Send + 'static,
    ) -> anyhow::Result<ProgressBar> {
        let hlist_pat![pb, hashed_sha256, temp_pourer_output] =
            Pipeline::build(stream, Arc::clone(&self.context))
                .fanout(operators.pb_updater)
                .fanout(operators.sha256_hasher)
                .fanout(operators.temp_pourer)
                .run_parallel()
                .await?;

        if hashed_sha256 != expected_sha256 {
            temp_pourer_output.cleanup().await?;

            let err =
                Self::streamed_sha256_mismatch_err(id, version, &hashed_sha256, expected_sha256);

            return Err(err);
        }

        temp_pourer_output.persist().await?;

        Ok(pb)
    }

    async fn stream_from_oci_or_url(
        &self,
        id: &str,
        version: &str,
        expected_sha256: &str,
        operators: Operators,
        stream: impl stream::Stream<Item = anyhow::Result<Bytes>> + Send + 'static,
    ) -> anyhow::Result<ProgressBar> {
        let hlist_pat![pb, hashed_sha256, temp_writer_output, temp_pourer_output] =
            Pipeline::build(stream, Arc::clone(&self.context))
                .fanout(operators.pb_updater)
                .fanout(operators.sha256_hasher)
                .fanout(operators.temp_writer)
                .fanout(operators.temp_pourer)
                .run_parallel()
                .await?;

        if hashed_sha256 != expected_sha256 {
            temp_writer_output.cleanup().await?;

            temp_pourer_output.cleanup().await?;

            let err =
                Self::streamed_sha256_mismatch_err(id, version, &hashed_sha256, expected_sha256);

            return Err(err);
        }

        temp_writer_output.persist().await?;

        temp_pourer_output.persist().await?;

        Ok(pb)
    }

    fn streamed_sha256_mismatch_err(
        id: &str,
        version: &str,
        actual: &str,
        expected: &str,
    ) -> anyhow::Error {
        anyhow!(formatdoc! {r#"
            SHA-256 mismatch detected for package "{id}" of version "{version}":

                - Actual    :   "{actual}"
                - Expected  :   "{expected}""#,
        })
    }

    async fn relocate(
        &self,
        streamed_package: &StreamedPackage,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        pb.set_prefix("Relocating");

        match streamed_package {
            StreamedPackage::Formula(streamed_formula) => {
                self.relocation.patch(streamed_formula).await?;
            },
            StreamedPackage::Cask(streamed_cask) => {
                self.artifact.relocate(streamed_cask).await?;
            },
        }

        Ok(())
    }

    async fn link(
        &self,
        streamed_package: &StreamedPackage,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        pb.set_prefix("Linking");

        self.linkers.link(streamed_package).await?;

        Ok(())
    }
}

struct Operators<TempWriter = push_operator::TempWriter> {
    pb_updater: push_operator::PbUpdater,
    sha256_hasher: push_operator::Sha256Hasher,
    temp_writer: TempWriter,
    temp_pourer: pull_operator::TempPourer,
}
