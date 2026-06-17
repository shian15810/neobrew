pub(crate) mod artifactor;
pub(crate) mod relocator;

use std::sync::Arc;

use async_trait::async_trait;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

use super::{
    Operator,
    state_committer::StateCommitter,
    state_store::{Payloads, Publish, Session, Stage, Subscribe},
};
use crate::{
    context::Context,
    package::prepared::{PreparedPackage, download::Download},
};

pub(crate) struct _SensorOperatorMarker;

#[async_trait]
pub(crate) trait SensorOperator: Sized {
    type Payload;
    type State;
    type Staging;
    type Output;

    fn poke_stage(&self) -> Stage;

    fn should_run(
        &self,
        _payload: Option<&Self::Payload>,
        _prepared_package: &PreparedPackage<Download>,
        _context: &Context,
    ) -> bool {
        true
    }

    fn on_skip_run(self) -> anyhow::Result<Option<Self::Output>> {
        Ok(None)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        None
    }

    fn init(&self, context: &Context) -> anyhow::Result<Self::State>;

    async fn execute(
        &self,
        state: &Self::State,
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

impl<SensorOp: SensorOperator<State: Send, Output: Send> + Send + 'static>
    Operator<_SensorOperatorMarker> for SensorOp
where
    Payloads: Subscribe<SensorOp::Payload> + Publish<SensorOp::Output>,
{
    type Input = SensorOp::Payload;
    type Output = SensorOp::Output;

    #[expect(clippy::let_and_return)]
    fn proceed(
        self,
        _: Option<Self::Input>,
        mut session: Session,
    ) -> AbortOnDropHandle<anyhow::Result<Option<Self::Output>>> {
        let handle = task::spawn(async move {
            let channel = &mut session.channel;

            let prepared_package = &session.prepared_package;

            let pb = &session.pb;

            let context = &session.context;

            let poke_stage = self.poke_stage();

            let payloads = {
                let state_store = channel
                    .state_store_rx
                    .wait_for(|state_store| state_store.stage >= poke_stage)
                    .await?;

                Arc::clone(&state_store.payloads)
            };

            let payload = payloads.subscribe()?;

            let should_run = self.should_run(payload, prepared_package, context);

            let state_committer = StateCommitter {
                passed_prefix: self.passed_prefix(),
                failed_prefix: self.failed_prefix(),

                passed_stage: self.passed_stage(should_run, prepared_package),
            };

            if !should_run {
                let output_res = self.on_skip_run();

                let output = state_committer.finalize(output_res, &session)?;

                return Ok(output);
            }

            if let Some(running_prefix) = self.running_prefix() {
                pb.set_prefix(running_prefix);
            }

            let state = self.init(context)?;

            let staging = self.execute(&state, prepared_package, context).await?;

            let output_res = self.on_final_run(staging);
            let output_res = output_res.map(Some);

            let output = state_committer.finalize(output_res, &session)?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        handle
    }
}
