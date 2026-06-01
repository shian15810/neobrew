pub(crate) trait AtomicWriter {
    fn cleanup(self) -> anyhow::Result<()>;

    async fn persist(self) -> anyhow::Result<()>;
}
