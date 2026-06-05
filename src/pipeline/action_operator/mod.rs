mod dmg_pourer;

use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use indicatif::ProgressBar;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

pub(crate) use self::dmg_pourer::DmgPourer;
use super::{
    Operator,
    state_store::{Channel, Publish, Stage},
};
use crate::context::Context;

#[async_trait]
pub(crate) trait ActionOperator: Sized {
    type Input;
    type Staging;
    type Output;

    async fn should_run(&self, _input: &Self::Input) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn on_skip_run(self) -> anyhow::Result<Self::Output> {
        let err = anyhow!("Implement `on_skip_run` when `should_run` returns `false`");

        Err(err)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        None
    }

    async fn execute(&self, input: &Self::Input) -> anyhow::Result<Self::Staging>;

    fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output>;

    fn persist(self) -> anyhow::Result<()> {
        Ok(())
    }

    fn cleanup(self) -> anyhow::Result<()> {
        Ok(())
    }

    fn passed_prefix(&self, _should_run: bool) -> Option<&'static str> {
        None
    }

    fn failed_prefix(&self, _should_run: bool) -> Option<&'static str> {
        None
    }

    fn passed_stage(&self, should_run: bool) -> Option<Stage>;
}

impl<ActionOp: ActionOperator<Input: Send, Output: Publish + Send> + Send + Sync + 'static> Operator
    for ActionOp
{
    type Input = ActionOp::Input;
    type Output = ActionOp::Output;

    #[expect(clippy::let_and_return)]
    fn proceed(
        self,
        input: Self::Input,
        pb: ProgressBar,
        channel: Channel,
        _context: Arc<Context>,
    ) -> AbortOnDropHandle<anyhow::Result<Self::Output>> {
        let handle = task::spawn(async move {
            let should_run = self.should_run(&input).await?;

            let action_runner = ActionRunner {
                passed_stage: self.passed_stage(should_run),

                passed_prefix: self.passed_prefix(should_run),
                failed_prefix: self.failed_prefix(should_run),
            };

            if !should_run {
                let output = self.on_skip_run();
                let output = action_runner.finalize(output, &pb, &channel)?;

                return Ok(output);
            }

            if let Some(running_prefix) = self.running_prefix() {
                pb.set_prefix(running_prefix);
            }

            let staging = self.execute(&input).await?;

            let output = self.on_final_run(staging);
            let output = action_runner.finalize(output, &pb, &channel)?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        handle
    }
}

struct ActionRunner {
    passed_stage: Option<Stage>,

    passed_prefix: Option<&'static str>,
    failed_prefix: Option<&'static str>,
}

impl ActionRunner {
    fn finalize<Output: Publish>(
        self,
        output: anyhow::Result<Output>,
        pb: &ProgressBar,
        channel: &Channel,
    ) -> anyhow::Result<Output> {
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
            let outputs = {
                let state_store = channel.state_store_rx.borrow();

                Arc::clone(&state_store.outputs)
            };

            output.publish(&outputs)?;

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
