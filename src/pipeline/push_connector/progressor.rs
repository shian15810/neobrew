use std::{fmt::Write as _, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::{
    super::state_store::{ProgressedOutput, Stage},
    PushConnector,
};
use crate::package::prepared::{Download, PreparedPackage, PreparedPackageable as _};

pub(crate) struct Progressor {
    pb: ProgressBar,
}

impl Progressor {
    const TEMPLATE_PREFIX: &str = "{spinner} {prefix:<12} {msg}";

    pub(crate) fn create(
        multi_pb: &MultiProgress,
        id: &str,
        version: &str,
        max_id_length: Option<usize>,
        max_version_length: Option<usize>,
    ) -> anyhow::Result<ProgressBar> {
        let pb = ProgressBar::new_spinner();
        let pb = multi_pb.add(pb);

        let mut template = Self::TEMPLATE_PREFIX.to_owned();

        template.push(' ');
        template.push_str("{wide_bar}");
        template.push(' ');
        template.push_str("({elapsed:>3})");

        let style = ProgressStyle::default_bar()
            .template(&template)?
            .progress_chars("  ");

        let mut message = String::new();

        match max_id_length {
            Some(max_id_length) => write!(message, "{id:<max_id_length$}")?,
            None => message.push_str(id),
        }

        message.push(' ');

        match max_version_length {
            Some(max_version_length) => write!(message, "{version:<max_version_length$}")?,
            None => message.push_str(version),
        }

        pb.set_style(style);

        pb.enable_steady_tick(Duration::from_millis(100));

        pb.set_message(message);

        Ok(pb)
    }

    pub(in super::super) fn new(pb: ProgressBar) -> Self {
        Self {
            pb,
        }
    }
}

#[async_trait]
impl PushConnector for Progressor {
    type State = ProgressBar;
    type Staging = ProgressBar;
    type Output = ProgressedOutput;

    fn running_prefix(&self) -> Option<&'static str> {
        Some("Streaming")
    }

    async fn init(
        &self,
        prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Self::State> {
        let download = prepared_package.download();

        let content_length = download.content_length();

        let mut template = Self::TEMPLATE_PREFIX.to_owned();

        template.push(' ');

        let style = if content_length.is_some() {
            template.push_str("[{wide_bar}]");
            template.push(' ');
            template.push_str("{decimal_bytes:>9}");
            template.push_str(" / ");
            template.push_str("{decimal_total_bytes:>9}");
            template.push(' ');
            template.push_str("({elapsed:>3})");

            ProgressStyle::default_bar()
                .template(&template)?
                .progress_chars("=> ")
        } else {
            template.push_str("{wide_bar}");
            template.push(' ');
            template.push_str("{decimal_bytes:>9}");
            template.push(' ');
            template.push_str("({elapsed:>3})");

            ProgressStyle::default_bar()
                .template(&template)?
                .progress_chars("  ")
        };

        let pb = self.pb.clone();

        pb.set_style(style);

        if let Some(content_length) = content_length {
            pb.set_length(content_length);
        }

        let state = pb;

        Ok(state)
    }

    async fn feed(&self, state: &mut Self::State, chunk: Bytes) -> anyhow::Result<()> {
        let pb = state;

        let content_length = chunk.len();
        let content_length = u64::try_from(content_length)?;

        pb.inc(content_length);

        Ok(())
    }

    async fn flush(&self, state: Self::State) -> anyhow::Result<Self::Staging> {
        let pb = state;

        let staging = pb;

        Ok(staging)
    }

    async fn on_final_run(
        self,
        staging: Self::Staging,
        _prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<Self::Output> {
        let pb = staging;

        let output = ProgressedOutput {
            position: pb.position(),
            length: pb.length(),
            per_sec: pb.per_sec(),
            elapsed: pb.elapsed(),
        };

        Ok(output)
    }

    fn passed_stage(&self, _should_run: bool) -> Option<Stage> {
        Some(Stage::Progressed)
    }
}
