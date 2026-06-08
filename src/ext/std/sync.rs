use std::sync::OnceLock;

use anyhow::{Context as _, anyhow};

pub(crate) trait OnceLockExt<T> {
    fn try_get(&self) -> anyhow::Result<&T>;

    fn try_set(&self, value: T) -> anyhow::Result<()>;
}

impl<T> OnceLockExt<T> for OnceLock<T> {
    fn try_get(&self) -> anyhow::Result<&T> {
        let value = self.get().context("`OnceLock` is still vacant")?;

        Ok(value)
    }

    fn try_set(&self, value: T) -> anyhow::Result<()> {
        if self.set(value).is_err() {
            let err = anyhow!("`OnceLock` is already occupied");

            return Err(err);
        }

        Ok(())
    }
}
