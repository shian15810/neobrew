use std::path::PathBuf;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::fs;

use super::{
    super::state_store::{ExtractedOutput, Stage, WrittenOutput},
    ActionOperator,
};
use crate::{
    context::Context,
    ext::tokio::path::PathExt as _,
    package::prepared::{PreparedPackage, PreparedPackageExt as _, download::Download},
    util::archive_format::ArchiveFormat,
};

pub(crate) struct PkgExtractor;

#[async_trait]
impl ActionOperator for PkgExtractor {
    type Input = WrittenOutput;
    type Staging = PathBuf;
    type Output = ExtractedOutput;

    async fn should_run(
        &self,
        _input: Option<&Self::Input>,
        prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<bool> {
        let download = prepared_package.download();

        let archive_format = download.archive_format();

        let is_pkg = self.is_pkg(archive_format);

        Ok(is_pkg)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        Some("Extracting")
    }

    async fn execute(
        &self,
        input: Option<&Self::Input>,
        prepared_package: &PreparedPackage<Download>,
        context: &Context,
    ) -> anyhow::Result<Self::Staging> {
        let Some(input) = input else {
            let err = anyhow!("`Input` is supposed to be defined");

            return Err(err);
        };

        let src_file_name = &input.dest_file_name;

        let src_file_path = &input.dest_file_path;

        let dest_dir_path = prepared_package.extract_dir_path(context);

        let dest_link_path = dest_dir_path.join(src_file_name);

        fs::create_dir_all(&dest_dir_path).await?;

        src_file_path
            .create_relative_link_atomically_at(dest_link_path)
            .await?;

        let staging = dest_dir_path;

        Ok(staging)
    }

    fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output> {
        let dest_dir_path = staging;

        let output = ExtractedOutput {
            dest_dir_path,

            archive_format: ArchiveFormat::Pkg,
        };

        Ok(output)
    }

    fn passed_stage(
        &self,
        should_run: bool,
        _prepared_package: &PreparedPackage<Download>,
    ) -> Option<Stage> {
        should_run.then_some(Stage::Extracted)
    }
}

impl PkgExtractor {
    #[expect(clippy::unused_self)]
    fn is_pkg(&self, archive_format: Option<ArchiveFormat>) -> bool {
        let Some(archive_format) = archive_format else {
            return false;
        };

        matches!(archive_format, ArchiveFormat::Pkg)
    }
}
