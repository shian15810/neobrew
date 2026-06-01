use std::{fmt::Write as _, time::Duration};

use bytes::Bytes;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::PushOperator;

pub(crate) struct PbUpdater {
    pb: ProgressBar,
}

impl PbUpdater {
    const TEMPLATE: &str = "{spinner} {prefix:<12} {msg}";

    pub(crate) fn create(
        multi_pb: &MultiProgress,
        id: &str,
        version: &str,
        max_id_length: Option<usize>,
        max_version_length: Option<usize>,
    ) -> anyhow::Result<ProgressBar> {
        let pb = ProgressBar::new_spinner();
        let pb = multi_pb.add(pb);

        let mut template = Self::TEMPLATE.to_owned();

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

    pub(crate) fn try_new(pb: ProgressBar, content_length: Option<u64>) -> anyhow::Result<Self> {
        let mut template = Self::TEMPLATE.to_owned();

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

        pb.set_style(style);

        pb.set_prefix("Streaming");

        if let Some(content_length) = content_length {
            pb.set_length(content_length);
        }

        let this = Self {
            pb,
        };

        Ok(this)
    }
}

impl PushOperator for PbUpdater {
    type Item = Bytes;
    type Output = ProgressBar;

    async fn feed(&mut self, chunk: Self::Item) -> anyhow::Result<()> {
        let content_length = chunk.len();
        let content_length = u64::try_from(content_length)?;

        self.pb.inc(content_length);

        Ok(())
    }

    async fn flush(self) -> anyhow::Result<Self::Output> {
        let pb = self.pb;

        Ok(pb)
    }
}
