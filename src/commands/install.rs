use std::{io::ErrorKind, sync::Arc};

use anyhow::{Result, anyhow};
use bytes::Bytes;
use clap::Args;
use frunk::hlist_pat;
use futures::stream::Stream;
use indoc::formatdoc;
use tokio::{fs::File as AsyncFile, task::JoinSet};
use tokio_util::io::ReaderStream;

use super::{Resolution, Runner};
use crate::{
    context::Context,
    ext::core::result::ResultExt as _,
    package::{
        Packageable as _,
        fetched::FetchedPackage,
        prepared::{PreparedPackage, PreparedPackageable as _},
        resolved::ResolvedPackage,
    },
    pipeline::{
        Pipeline,
        handler::AtomicWriter as _,
        pull_operator::{self, TempPourerInput},
        push_operator::{self, TempWriterInput},
    },
    registry::Registry,
    utils::Linker,
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
        if self.packages.is_empty() {
            return Ok(());
        }

        let resolved_packages = self.resolve_packages(Arc::clone(&context)).await?;

        let prepared_packages = Self::prepare_packages(resolved_packages)?;

        let linker = Linker::create(Arc::clone(&context)).await?;
        let linker = Arc::new(linker);

        let mut set = JoinSet::new();

        for prepared_package in prepared_packages {
            while set.len() >= *context.concurrency_limit {
                if let Some(res) = set.join_next().await {
                    res??;
                }
            }

            let context = Arc::clone(&context);

            let linker = Arc::clone(&linker);

            set.spawn(async move {
                let fetched_package =
                    Self::fetch_package(prepared_package, Arc::clone(&context)).await?;

                let Some(fetched_package) = fetched_package else {
                    return Ok(());
                };

                let () = Self::install_package(fetched_package, &linker, &context).await?;

                anyhow::Ok(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}

impl Install {
    async fn resolve_packages(self, context: Arc<Context>) -> Result<Vec<ResolvedPackage>> {
        let registry = Registry::new(context);

        let strategy = self.resolution.strategy();

        let resolved_packages = registry.resolve(self.packages, strategy).await?;

        Ok(resolved_packages)
    }

    fn prepare_packages(resolved_packages: Vec<ResolvedPackage>) -> Result<Vec<PreparedPackage>> {
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

        Ok(prepared_packages)
    }

    async fn fetch_package(
        prepared_package: PreparedPackage,
        context: Arc<Context>,
    ) -> Result<Option<FetchedPackage>> {
        let id = prepared_package.id();

        let version = prepared_package.version();

        let keg_dir_path = context.homebrew_dirs.keg_dir(id, version);

        if keg_dir_path.is_dir() {
            let fetched_package = FetchedPackage::from(prepared_package);

            return Ok(Some(fetched_package));
        }

        let expected_sha256 = prepared_package.expected_sha256();

        let temp_writer_input = prepared_package.temp_writer_input(&context).await?;

        let cache_file_path = &temp_writer_input.file_path;

        if let Some(cache_symlink_path) = &temp_writer_input.symlink_path
            && cache_symlink_path.is_symlink()
            && cache_file_path.is_file()
            && cache_symlink_path.canonicalize()? == cache_file_path.canonicalize()?
        {
            let cache_file_sha256 = prepared_package.cache_file_sha256(cache_file_path).await?;

            if let Some(cache_file_sha256) = cache_file_sha256
                && cache_file_sha256 == expected_sha256
            {
                let temp_pourer_input = prepared_package.temp_pourer_input(&context);

                let fetch_package_from_cache = Self::fetch_package_from_cache(
                    id,
                    version,
                    expected_sha256,
                    temp_writer_input,
                    temp_pourer_input,
                    context,
                )
                .await?;

                let Some(()) = fetch_package_from_cache else {
                    return Ok(None);
                };

                let fetched_package = FetchedPackage::from(prepared_package);

                return Ok(Some(fetched_package));
            }
        }

        let Some(stream) = prepared_package.stream(&context).await? else {
            return Ok(None);
        };

        let temp_pourer_input = prepared_package.temp_pourer_input(&context);

        Self::fetch_package_from_url(
            id,
            version,
            expected_sha256,
            temp_writer_input,
            temp_pourer_input,
            stream,
            context,
        )
        .await?;

        let fetched_package = FetchedPackage::from(prepared_package);

        Ok(Some(fetched_package))
    }

    async fn fetch_package_from_cache(
        id: &str,
        version: &str,
        expected_sha256: &str,
        temp_writer_input: TempWriterInput,
        temp_pourer_input: TempPourerInput,
        context: Arc<Context>,
    ) -> Result<Option<()>> {
        let cache_file = match AsyncFile::open(temp_writer_input.file_path).await {
            Ok(cache_file) => cache_file,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err)?,
        };

        let stream = ReaderStream::new(cache_file);

        let hasher = push_operator::Hasher::new();

        let temp_pourer = pull_operator::TempPourer::create(temp_pourer_input);

        let hlist_pat![hashed_sha256, temp_pourer_output] = Pipeline::new(stream, context)
            .fanout(hasher)
            .fanout(temp_pourer)
            .run_parallel()
            .await?;

        if hashed_sha256 != expected_sha256 {
            temp_pourer_output.cleanup().await?;

            let err = Self::fetch_package_err_of_hashed_sha256_mismatch(
                id,
                version,
                &hashed_sha256,
                expected_sha256,
            );

            return Err(err);
        }

        temp_pourer_output.persist().await?;

        Ok(Some(()))
    }

    async fn fetch_package_from_url(
        id: &str,
        version: &str,
        expected_sha256: &str,
        temp_writer_input: TempWriterInput,
        temp_pourer_input: TempPourerInput,
        stream: impl Stream<Item = Result<Bytes>> + Send + 'static,
        context: Arc<Context>,
    ) -> Result<()> {
        let hasher = push_operator::Hasher::new();

        let temp_writer = push_operator::TempWriter::create(temp_writer_input).await?;

        let temp_pourer = pull_operator::TempPourer::create(temp_pourer_input);

        let hlist_pat![hashed_sha256, temp_writer_output, temp_pourer_output] =
            Pipeline::new(stream, context)
                .fanout(hasher)
                .fanout(temp_writer)
                .fanout(temp_pourer)
                .run_parallel()
                .await?;

        if hashed_sha256 != expected_sha256 {
            temp_writer_output.cleanup().await?;

            temp_pourer_output.cleanup().await?;

            let err = Self::fetch_package_err_of_hashed_sha256_mismatch(
                id,
                version,
                &hashed_sha256,
                expected_sha256,
            );

            return Err(err);
        }

        temp_writer_output.persist().await?;

        temp_pourer_output.persist().await?;

        Ok(())
    }

    fn fetch_package_err_of_hashed_sha256_mismatch(
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

    async fn install_package(
        fetched_package: FetchedPackage,
        linker: &Linker,
        context: &Context,
    ) -> Result<()> {
        match fetched_package {
            FetchedPackage::Formula(fetched_formula) => {
                fetched_formula.relocate(context).await?;

                fetched_formula.link(linker).await?;

                Ok(())
            },
            FetchedPackage::Cask(_fetched_cask) => Ok(()),
        }
    }
}
