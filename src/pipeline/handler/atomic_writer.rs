use anyhow::Result;

pub(crate) trait AtomicWriter {
    async fn cleanup(self) -> Result<()>;

    async fn persist(self) -> Result<()>;
}
