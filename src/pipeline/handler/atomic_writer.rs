pub(crate) trait AtomicWriter {
    async fn cleanup(self) -> anyhow::Result<()>;

    async fn persist(self) -> anyhow::Result<()>;
}
