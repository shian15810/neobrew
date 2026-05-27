use std::{
    fmt::Write as _,
    sync::{Arc, Mutex, PoisonError},
};

use anyhow::{Context as _, Result, anyhow};
use bytes::Bytes;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::PushOperator;

pub(crate) struct Progress {
    pb: ProgressBar,
    active_pbs: Arc<Mutex<Vec<(String, ProgressBar)>>>,

    id: String,
}

impl Progress {
    pub(crate) fn create(
        multi_pb: &MultiProgress,
        active_pbs: Arc<Mutex<Vec<(String, ProgressBar)>>>,
        id: &str,
        version: &str,
        max_id_length: Option<usize>,
        max_version_length: Option<usize>,
        content_length: Option<u64>,
    ) -> Result<Self> {
        let pb = {
            let mut active_pbs = active_pbs.lock().unwrap_or_else(PoisonError::into_inner);

            let Err(index) = active_pbs.binary_search_by_key(&id, |(id, _)| id) else {
                let err = anyhow!(r#"Progress bar "{id}" already exists in `active_pbs`"#);

                return Err(err);
            };

            let pb = if let Some(content_length) = content_length {
                ProgressBar::new(content_length)
            } else {
                ProgressBar::no_length()
            };

            let pb = match index.checked_sub(1) {
                None => multi_pb.insert(0, pb),
                Some(index) => {
                    let (_, after_pb) = active_pbs.get(index).context("Index out of bounds")?;

                    multi_pb.insert_after(after_pb, pb)
                },
            };

            active_pbs.insert(index, (id.to_owned(), pb.clone()));

            pb
        };

        let pre_template = "{prefix:.bold} {msg} [{elapsed_precise}]";

        if content_length.is_some() {
            let post_template = "[{wide_bar}] {bytes}/{total_bytes} ({eta})";

            let template = format!("{pre_template} {post_template}");

            pb.set_style(
                ProgressStyle::default_bar()
                    .template(&template)?
                    .progress_chars("=> "),
            );
        } else {
            let post_template = "{spinner} {bytes} ({eta})";

            let template = format!("{pre_template} {post_template}");

            pb.set_style(ProgressStyle::default_spinner().template(&template)?);
        }

        pb.set_prefix("Streaming");

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

        pb.set_message(message);

        let this = Self {
            pb,
            active_pbs,

            id: id.to_owned(),
        };

        Ok(this)
    }
}

impl PushOperator for Progress {
    type Item = Bytes;
    type Output = ();

    async fn feed(&mut self, chunk: Self::Item) -> Result<()> {
        let content_length = chunk.len();
        let content_length = u64::try_from(content_length)?;

        self.pb.inc(content_length);

        Ok(())
    }

    async fn flush(self) -> Result<Self::Output> {
        self.pb.set_prefix("Installed");

        self.pb.finish();

        let mut active_pbs = self
            .active_pbs
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        let Ok(index) = active_pbs.binary_search_by_key(&&self.id, |(id, _)| id) else {
            let id = self.id;

            let err = anyhow!(r#"Progress bar "{id}" not found in `active_pbs`"#);

            return Err(err);
        };

        active_pbs.remove(index);

        Ok(())
    }
}
