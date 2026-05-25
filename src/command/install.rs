use std::sync::Arc;

use anyhow::{Result, anyhow};
use clap::Args;
use frunk::hlist_pat;
use indoc::formatdoc;
use tokio::{fs::File, task::JoinSet};
use tokio_util::io::ReaderStream;

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
    registries::Registries,
    relocation::Relocation,
};

#[derive(Args)]
pub(super) struct Install {
    #[command(flatten)]
    resolution: Resolution,

    #[arg(value_name = "PACKAGE")]
    packages: Vec<String>,
}

impl Runner for Install {
    #[expect(clippy::too_many_lines)]
    async fn run_concurrent(self, context: Arc<Context>) -> Result<()> {
        if self.packages.is_empty() {
            return Ok(());
        }

        let strategy = self.resolution.strategy();

        let registries = Registries::new(Arc::clone(&context));

        let resolved_packages = registries.resolve(self.packages, strategy).await?;

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

        let caches = Caches::new(Arc::clone(&context));
        let caches = Arc::new(caches);

        let relocation = Relocation::from(&context.homebrew_dirs);
        let relocation = Arc::new(relocation);

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

            let caches = Arc::clone(&caches);

            let relocation = Arc::clone(&relocation);

            let linker = Arc::clone(&linker);

            set.spawn(async move {
                let id = prepared_package.id();

                let version = prepared_package.version();

                let keg_dir_path = context.homebrew_dirs.keg_dir(id, version);

                #[expect(clippy::disallowed_macros)]
                if keg_dir_path.is_dir_exists_nofollow().await? {
                    unimplemented!();
                }

                let expected_sha256 = prepared_package.expected_sha256();

                let package_cache = caches.retrieve(&prepared_package, expected_sha256).await?;

                if package_cache.is_valid {
                    let download_file = File::open(&package_cache.file_path).await?;

                    let stream = ReaderStream::new(download_file);

                    let hasher = push_operator::Hasher::new();

                    let pourer_dir_path = match prepared_package {
                        PreparedPackage::Formula(_) => context.homebrew_dirs.cellar_dir(),
                        PreparedPackage::Cask(_) => context.homebrew_dirs.caskroom_dir(),
                    };

                    let temp_pourer = pull_operator::TempPourer::create(pourer_dir_path);

                    let hlist_pat![hashed_sha256, temp_pourer_output] =
                        Pipeline::new(stream, Arc::clone(&context))
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
                }

                let hasher = push_operator::Hasher::new();

                let temp_writer = push_operator::TempWriter::create(
                    package_cache.file_path,
                    vec![package_cache.symlink_path],
                )
                .await?;

                let pourer_dir_path = match prepared_package {
                    PreparedPackage::Formula(_) => context.homebrew_dirs.cellar_dir(),
                    PreparedPackage::Cask(_) => context.homebrew_dirs.caskroom_dir(),
                };

                let temp_pourer = pull_operator::TempPourer::create(pourer_dir_path);

                let Some(stream) = prepared_package.stream(&context).await? else {
                    return Ok(());
                };

                let hlist_pat![hashed_sha256, temp_writer_output, temp_pourer_output] =
                    Pipeline::new(stream, Arc::clone(&context))
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

                let fetched_package = FetchedPackage::from(prepared_package);

                match fetched_package {
                    FetchedPackage::Formula(fetched_formula) => {
                        if fetched_formula.should_relocate() {
                            let keg_dir_path = context
                                .homebrew_dirs
                                .keg_dir(fetched_formula.id(), fetched_formula.version());

                            relocation.patch_keg(&keg_dir_path).await?;
                        }

                        linker.link_opt(&fetched_formula).await?;

                        if fetched_formula.should_link_keg() {
                            linker.link_keg(&fetched_formula).await?;
                        }
                    },
                    FetchedPackage::Cask(_fetched_cask) => {},
                }

                Ok(())
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}

impl Install {
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
}
