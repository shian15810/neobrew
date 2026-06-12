mod dmg_pourer;
mod linker;

use async_trait::async_trait;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

pub(crate) use self::{dmg_pourer::DmgPourer, linker::Linker};
use super::{
    Operator,
    state_committer::StateCommitter,
    state_store::{Payloads, Publish, Session, Stage},
};
use crate::{
    context::Context,
    package::prepared::{Download, PreparedPackage},
};

pub(crate) struct _ActionOperatorMarker;

#[async_trait]
pub(crate) trait ActionOperator: Sized {
    type Input: Sync;
    type Staging;
    type Output;

    async fn should_run(
        &self,
        _input: Option<&Self::Input>,
        _prepared_package: &PreparedPackage<Download>,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn on_skip_run(self) -> anyhow::Result<Option<Self::Output>> {
        Ok(None)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        None
    }

    async fn execute(
        &self,
        input: Option<&Self::Input>,
        prepared_package: &PreparedPackage<Download>,
        context: &Context,
    ) -> anyhow::Result<Self::Staging>;

    fn on_final_run(self, staging: Self::Staging) -> anyhow::Result<Self::Output>;

    fn passed_prefix(&self) -> Option<&'static str> {
        None
    }

    fn failed_prefix(&self) -> Option<&'static str> {
        None
    }

    fn passed_stage(
        &self,
        should_run: bool,
        prepared_package: &PreparedPackage<Download>,
    ) -> Option<Stage>;
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
        input: Option<Self::Input>,
        mut session: Session,
    ) -> AbortOnDropHandle<anyhow::Result<Option<Self::Output>>> {
        let handle = task::spawn(async move {
            let _channel = &mut session.channel;

            let prepared_package = &session.prepared_package;

            let pb = &session.pb;

            let context = &session.context;

            let should_run = self.should_run(input.as_ref(), prepared_package).await?;

            let state_committer = StateCommitter {
                passed_prefix: self.passed_prefix(),
                failed_prefix: self.failed_prefix(),

                passed_stage: self.passed_stage(should_run, prepared_package),
            };

            if !should_run {
                let output = self.on_skip_run();
                let output = state_committer.finalize(output, &session)?;

                return Ok(output);
            }

            if let Some(running_prefix) = self.running_prefix() {
                pb.set_prefix(running_prefix);
            }

            let staging = self
                .execute(input.as_ref(), prepared_package, context)
                .await?;

            let output = self.on_final_run(staging);
            let output = output.map(Some);
            let output = state_committer.finalize(output, &session)?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        handle
    }
}
