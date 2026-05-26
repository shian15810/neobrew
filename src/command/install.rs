use std::sync::Arc;

use anyhow::{Result, anyhow};
use bytes::Bytes;
use clap::Args;
use frunk::hlist_pat;
use futures::stream;
use indoc::formatdoc;
use tokio::task::JoinSet;

use super::{Resolution, Runner};
use crate::{
    caches::Caches,
    context::Context,
    ext::{core::result::ResultExt as _, tokio::path::PathExt as _},
    linker::Linker,
    package::{
        Packageable as _,
        fetched::FetchedPackage,
        prepared::{PreparedPackage, PreparedPackageable as _},
    },
    pipeline::{Pipeline, handler::AtomicWriter as _, pull_operator, push_operator},
    registries::{Registries, ResolutionStrategy},
    relocation::Relocation,
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
    async fn run_concurrent(self, context: Arc<Context>) -> Result<()> {
        let installation = Installation::init(context).await?;

        installation.run(self.packages, self.resolution).await?;

        Ok(())
    }
}

struct Installation {
    context: Arc<Context>,

    caches: Caches,
    streams: Streams,
    relocation: Relocation,
    linker: Linker,
}

impl Installation {
    async fn init(context: Arc<Context>) -> Result<Arc<Self>> {
        let caches = Caches::new(Arc::clone(&context));

        let streams = Streams::new(Arc::clone(&context));

        let relocation = Relocation::from(&context.homebrew_dirs);

        let linker = Linker::create(Arc::clone(&context)).await?;

        let this = Self {
            context,

            caches,
            streams,
            relocation,
            linker,
        };
        let this = Arc::new(this);

        Ok(this)
    }

    async fn run(self: Arc<Self>, packages: Vec<String>, resolution: Resolution) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let strategy = resolution.strategy();

        self.run_many(packages, strategy).await?;

        Ok(())
    }

    async fn run_many(
        self: Arc<Self>,
        packages: Vec<String>,
        strategy: ResolutionStrategy,
    ) -> Result<()> {
        let registries = Registries::new(Arc::clone(&self.context));

        let resolved_packages = registries.resolve(packages, strategy).await?;

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
            .collect::<Result<Vec<_>>>()?;

        let mut set = JoinSet::new();

        for prepared_package in prepared_packages {
            while set.len() >= *self.context.concurrency_limit {
                if let Some(res) = set.join_next().await {
                    res??;
                }
            }

            let this = Arc::clone(&self);

            set.spawn(async move {
                this.run_one(prepared_package).await?;

                anyhow::Ok(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }

    async fn run_one(&self, prepared_package: PreparedPackage) -> Result<()> {
        let id = prepared_package.id();

        let version = prepared_package.version();

        let expected_sha256 = prepared_package.expected_sha256();

        let keg_dir_path = self.context.homebrew_dirs.keg_dir(id, version);

        if keg_dir_path.is_dir_exists_nofollow().await? {
            let fetched_package = FetchedPackage::from(prepared_package);

            self.relocate(&fetched_package).await?;

            self.link(&fetched_package).await?;

            return Ok(());
        }

        let cache = self
            .caches
            .retrieve(&prepared_package, expected_sha256)
            .await?;

        let Some(archive_format) = cache.archive_format else {
            let err = anyhow!(r#"Archive file format of package "{id}" is not yet supported"#);

            return Err(err);
        };

        let pourer_dir_path = match prepared_package {
            PreparedPackage::Formula(_) => self.context.homebrew_dirs.cellar_dir(),
            PreparedPackage::Cask(_) => self.context.homebrew_dirs.caskroom_dir(),
        };

        let temp_pourer =
            pull_operator::TempPourer::create(archive_format, pourer_dir_path, vec![]);

        if cache.is_valid {
            let cache_stream = self.streams.cache(&cache.file_path).await?;

            self.fetch_from_cache(id, version, expected_sha256, temp_pourer, cache_stream)
                .await?;
        } else {
            let temp_writer =
                push_operator::TempWriter::create(cache.file_path, vec![cache.symlink_path])
                    .await?;

            let Some(api_stream) = self.streams.api(&prepared_package).await? else {
                return Ok(());
            };

            self.fetch_from_api(
                id,
                version,
                expected_sha256,
                temp_pourer,
                temp_writer,
                api_stream,
            )
            .await?;
        }

        let fetched_package = FetchedPackage::from(prepared_package);

        self.relocate(&fetched_package).await?;

        self.link(&fetched_package).await?;

        Ok(())
    }

    async fn fetch_from_cache(
        &self,
        id: &str,
        version: &str,
        expected_sha256: &str,
        temp_pourer: pull_operator::TempPourer,
        cache_stream: impl stream::Stream<Item = Result<Bytes>> + Send + 'static,
    ) -> Result<()> {
        let hasher = push_operator::Hasher::new();

        let hlist_pat![temp_pourer_output, hashed_sha256] =
            Pipeline::new(cache_stream, Arc::clone(&self.context))
                .fanout(temp_pourer)
                .fanout(hasher)
                .run_parallel()
                .await?;

        if hashed_sha256 != expected_sha256 {
            temp_pourer_output.cleanup().await?;

            let err = Self::fetch_sha256_mismatch_err(id, version, &hashed_sha256, expected_sha256);

            return Err(err);
        }

        temp_pourer_output.persist().await?;

        Ok(())
    }

    async fn fetch_from_api(
        &self,
        id: &str,
        version: &str,
        expected_sha256: &str,
        temp_pourer: pull_operator::TempPourer,
        temp_writer: push_operator::TempWriter,
        api_stream: impl stream::Stream<Item = Result<Bytes>> + Send + 'static,
    ) -> Result<()> {
        let hasher = push_operator::Hasher::new();

        let hlist_pat![temp_pourer_output, temp_writer_output, hashed_sha256] =
            Pipeline::new(api_stream, Arc::clone(&self.context))
                .fanout(temp_pourer)
                .fanout(temp_writer)
                .fanout(hasher)
                .run_parallel()
                .await?;

        if hashed_sha256 != expected_sha256 {
            temp_writer_output.cleanup().await?;

            temp_pourer_output.cleanup().await?;

            let err = Self::fetch_sha256_mismatch_err(id, version, &hashed_sha256, expected_sha256);

            return Err(err);
        }

        temp_writer_output.persist().await?;

        temp_pourer_output.persist().await?;

        Ok(())
    }

    fn fetch_sha256_mismatch_err(
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

    async fn relocate(&self, fetched_package: &FetchedPackage) -> Result<()> {
        match fetched_package {
            FetchedPackage::Formula(fetched_formula) => {
                if fetched_formula.should_relocate() {
                    let keg_dir_path = self
                        .context
                        .homebrew_dirs
                        .keg_dir(fetched_formula.id(), fetched_formula.version());

                    self.relocation.patch_keg(&keg_dir_path).await?;
                }
            },
            FetchedPackage::Cask(_fetched_cask) => {},
        }

        Ok(())
    }

    async fn link(&self, fetched_package: &FetchedPackage) -> Result<()> {
        match fetched_package {
            FetchedPackage::Formula(fetched_formula) => {
                self.linker.link_opt(fetched_formula).await?;

                if fetched_formula.should_link_keg() {
                    self.linker.link_keg(fetched_formula).await?;
                }
            },
            FetchedPackage::Cask(_fetched_cask) => {},
        }

        Ok(())
    }
}
