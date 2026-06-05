use std::sync::OnceLock;

use anyhow::anyhow;

pub(crate) trait OnceLockExt<T> {
    fn try_set(&self, value: T) -> anyhow::Result<()>;
}

impl<T> OnceLockExt<T> for OnceLock<T> {
    fn try_set(&self, value: T) -> anyhow::Result<()> {
        self.set(value)
            .map_err(|_| anyhow!("`OnceLock` is already occupied"))?;

        Ok(())
    }
}
