use std::sync::Arc;

use super::state_store::{Payloads, Publish, Session, Stage};

pub(super) struct StateCommitter {
    pub(super) passed_stage: Option<Stage>,

    pub(super) passed_prefix: Option<&'static str>,
    pub(super) failed_prefix: Option<&'static str>,
}

impl StateCommitter {
    pub(super) fn finalize<Output>(
        self,
        output: anyhow::Result<Output>,
        session: &Session,
    ) -> anyhow::Result<Output>
    where
        Payloads: Publish<Output>,
    {
        let channel = &session.channel;

        let pb = &session.pb;

        let output = match output {
            Ok(output) => {
                if let Some(passed_prefix) = self.passed_prefix {
                    pb.set_prefix(passed_prefix);
                }

                output
            },
            Err(err) => {
                if let Some(failed_prefix) = self.failed_prefix {
                    pb.set_prefix(failed_prefix);
                }

                pb.finish();

                return Err(err);
            },
        };

        if let Some(passed_stage) = self.passed_stage {
            let payloads = {
                let state_store = channel.state_store_rx.borrow();

                Arc::clone(&state_store.payloads)
            };

            payloads.publish(&output)?;

            channel.state_store_tx.send_if_modified(|state_store| {
                passed_stage > state_store.stage && {
                    state_store.stage = passed_stage;

                    true
                }
            });
        }

        Ok(output)
    }
}
