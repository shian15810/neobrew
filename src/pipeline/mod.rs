use anyhow::Result;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use tokio::{sync::mpsc, task::JoinSet};

use self::operator::Operator;

pub mod operator;

pub struct Pipeline {
    txs: Vec<mpsc::Sender<Bytes>>,
    set: JoinSet<Result<()>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            txs: Vec::new(),
            set: JoinSet::new(),
        }
    }

    pub fn broadcast(mut self, mut operator: impl Operator) -> Self {
        let (tx, mut rx) = mpsc::channel(32);

        self.txs.push(tx);

        self.set.spawn_blocking(move || {
            while let Some(chunk) = rx.blocking_recv() {
                operator.send(chunk)?;
            }

            operator.apply()
        });

        self
    }

    pub async fn apply(
        mut self,
        mut stream: impl Stream<Item = reqwest::Result<Bytes>> + Unpin,
    ) -> Result<()> {
        while let Some(item) = stream.next().await {
            let chunk = item?;

            for tx in &self.txs {
                tx.send(chunk.clone()).await?;
            }
        }

        // Drop senders to close channels and terminate worker threads.
        self.txs.clear();

        while let Some(res) = self.set.join_next().await {
            res??;
        }

        Ok(())
    }
}
