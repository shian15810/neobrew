use std::sync::Arc;

use anyhow::anyhow;
use tokio::task;
use tokio_util::task::AbortOnDropHandle;

use super::{
    Operator,
    state_committer::StateCommitter,
    state_store::{Payloads, Publish, Session, Stage, Subscribe},
};

struct _SensorOperatorMarker;

trait SensorOperator: Sized {
    type Payload;
    type Staging;
    type Output;

    fn should_run(&self) -> bool {
        true
    }

    fn on_skip_run(self) -> anyhow::Result<Self::Output> {
        let err = anyhow!("Implement `on_skip_run` when `should_run` returns `false`");

        Err(err)
    }

    fn running_prefix(&self) -> Option<&'static str> {
        None
    }

    fn poke_stage(&self) -> Stage;

    fn execute(&self, payload: &Self::Payload) -> anyhow::Result<Self::Staging>;

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

impl<SensorOp: SensorOperator<Output: Send> + Send + 'static> Operator<_SensorOperatorMarker>
    for SensorOp
where
    Payloads: Subscribe<SensorOp::Payload> + Publish<SensorOp::Output>,
{
    type Input = ();
    type Output = SensorOp::Output;

    #[expect(clippy::let_and_return)]
    fn proceed(
        self,
        (): Self::Input,
        mut session: Session,
    ) -> AbortOnDropHandle<anyhow::Result<Self::Output>> {
        let handle = task::spawn(async move {
            let channel = &mut session.channel;

            let pb = &session.pb;

            let should_run = self.should_run();

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

            let poke_stage = self.poke_stage();

            let payloads = {
                let state_store = channel
                    .state_store_rx
                    .wait_for(|state_store| state_store.stage >= poke_stage)
                    .await?;

                Arc::clone(&state_store.payloads)
            };

            let payload = payloads.subscribe()?;

            if let Some(running_prefix) = self.running_prefix() {
                pb.set_prefix(running_prefix);
            }

            let staging = self.execute(payload)?;

            let output = self.on_final_run(staging);
            let output = state_committer.finalize(output, &session)?;

            anyhow::Ok(output)
        });
        let handle = AbortOnDropHandle::new(handle);

        handle
    }
}
