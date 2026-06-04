use std::sync::Arc;

use crate::context::Context;

pub(super) struct CaskStream {
    context: Arc<Context>,
}

impl CaskStream {
    pub(super) fn new(context: Arc<Context>) -> Self {
        Self {
            context,
        }
    }
}
