mod dmg_pourer;

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

pub(crate) use self::dmg_pourer::DmgPourer;
use super::{
    Operator,
    state_committer::StateCommitter,
    state_store::{Payloads, Publish, Session, Stage},
};

pub(crate) struct _ActionOperatorMarker;

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

impl<ActionOp: ActionOperator<Input: Send, Output: Send> + Send + Sync + 'static>
    Operator<_ActionOperatorMarker> for ActionOp
where
    Payloads: Publish<ActionOp::Output>,
{
    type Input = ActionOp::Input;
    type Output = ActionOp::Output;

    #[expect(clippy::let_and_return)]
    fn proceed(
        self,
        input: Self::Input,
        session: Session,
    ) -> AbortOnDropHandle<anyhow::Result<Self::Output>> {
        let handle = task::spawn(async move {
            let pb = &session.pb;

            let should_run = self.should_run(&input).await?;

            let state_committer = StateCommitter {
                passed_stage: self.passed_stage(should_run),

                passed_prefix: self.passed_prefix(should_run),
                failed_prefix: self.failed_prefix(should_run),
            };

            if !should_run {
                let output = self.on_skip_run();
                let output = state_committer.finalize(output, &session)?;

                return Ok(output);
            }

            if let Some(running_prefix) = self.running_prefix() {
                pb.set_prefix(running_prefix);
            }

            let staging = self.execute(&input).await?;

            let output = self.on_final_run(staging);
            let output = state_committer.finalize(output, &session)?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        handle
    }
}
