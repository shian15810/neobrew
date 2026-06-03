mod dmg_pourer;

use std::sync::Arc;

pub(crate) use self::dmg_pourer::DmgPourer;
use super::channels::PipelineChannels as Channels;
use crate::context::Context;

#[trait_variant::make(Send)]
pub(crate) trait PostOperator {
    type Input;
    type Output;

    async fn proceed(
        self,
        input: Self::Input,
        channels: Arc<Channels>,
        context: Arc<Context>,
    ) -> anyhow::Result<Self::Output>;
}
