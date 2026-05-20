use std::sync::Arc;

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
    ext::ResultExt as _,
    package::{
        FetchedPackage,
        Packageable as _,
        PreparedPackage,
        PreparedPackageCache,
        PreparedPackageDest,
        PreparedPackageable as _,
    },
    pipeline::{AtomicFsHandler as _, Hasher, Pipeline, Pourer, Writer},
    registry::Registry,
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

        let registry = {
            let context = Arc::clone(&context);

            Registry::new(context)
        };

        let strategy = self.resolution.strategy();

        let resolved_packages = registry.resolve(self.packages, strategy).await?;

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
            while set.len() >= *context.concurrency_limit {
                if let Some(res) = set.join_next().await {
                    res??;
                }
            }

            let context = Arc::clone(&context);

            set.spawn(async move {
                let id = prepared_package.id();

                let version = prepared_package.version();

                let dest = {
                    let context = Arc::as_ref(&context);

                    prepared_package.dest(context)
                };

                if dest.dir_location.is_dir() {
                    let fetched_package = FetchedPackage::from(prepared_package);

                    return Ok(Some(fetched_package));
                }

                let cache = {
                    let context = Arc::as_ref(&context);

                    prepared_package.cache(context).await?
                };

                let sha256 = prepared_package.sha256();

                if cache.symlink_location.is_symlink()
                    && cache.file_location.is_file()
                    && cache.symlink_location.canonicalize()?
                        == cache.file_location.canonicalize()?
                {
                    let cache_file_sha256 = cache.file_sha256().await?;

                    if cache_file_sha256 == sha256 {
                        Self::fetch_from_cache(id, version, dest, cache, sha256, context).await?;

                        let fetched_package = FetchedPackage::from(prepared_package);

                        return Ok(Some(fetched_package));
                    }
                }

                let stream = {
                    let context = Arc::as_ref(&context);

                    prepared_package.stream(context).await?
                };

                let Some(stream) = stream else {
                    return Ok(None);
                };

                Self::fetch_from_url(id, version, dest, cache, sha256, stream, context).await?;

                let fetched_package = FetchedPackage::from(prepared_package);

                anyhow::Ok(Some(fetched_package))
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}

impl Install {
    async fn fetch_from_cache(
        id: &str,
        version: &str,
        dest: PreparedPackageDest,
        cache: PreparedPackageCache,
        sha256: &str,
        context: Arc<Context>,
    ) -> Result<()> {
        let cache_file = AsyncFile::open(cache.file_location).await?;

        let stream = ReaderStream::new(cache_file);

        let pourer = Pourer::from(dest.clone());

        let hasher = Hasher::new();

        let hlist_pat![poured_temp_dest, hashed_temp_sha256] = Pipeline::new(stream, context)
            .fanout(pourer)
            .fanout(hasher)
            .run_parallel()
            .await?;

        if hashed_temp_sha256 != sha256 {
            poured_temp_dest.cleanup().await?;

            let err = Self::sha256_mismatch_err(id, version, &hashed_temp_sha256, sha256);

            return Err(err);
        }

        poured_temp_dest.persist().await?;

        Ok(())
    }

    async fn fetch_from_url(
        id: &str,
        version: &str,
        dest: PreparedPackageDest,
        cache: PreparedPackageCache,
        sha256: &str,
        stream: impl Stream<Item = Result<Bytes>> + Send + 'static,
        context: Arc<Context>,
    ) -> Result<()> {
        let pourer = Pourer::from(dest);

        let writer = Writer::create(cache).await?;

        let hasher = Hasher::new();

        let hlist_pat![poured_temp_dest, written_temp_cache, hashed_temp_sha256] =
            Pipeline::new(stream, context)
                .fanout(pourer)
                .fanout(writer)
                .fanout(hasher)
                .run_parallel()
                .await?;

        if hashed_temp_sha256 != sha256 {
            poured_temp_dest.cleanup().await?;

            written_temp_cache.cleanup().await?;

            let err = Self::sha256_mismatch_err(id, version, &hashed_temp_sha256, sha256);

            return Err(err);
        }

        poured_temp_dest.persist().await?;

        written_temp_cache.persist().await?;

        Ok(())
    }

    fn sha256_mismatch_err(id: &str, version: &str, actual: &str, expected: &str) -> anyhow::Error {
        anyhow!(formatdoc! {r#"
            SHA-256 mismatch detected for package "{id}" of version "{version}":

            Actual  : "{actual}"
            Expected: "{expected}""#,
        })
    }
}
