use async_broadcast::Sender;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::consumer::ExperimentDocument;

/// Generate 60 request batches
///
/// The batches are stored on a shared queue, wherein the receiver appends to the queue, and the
/// executors pop from the queue (FIFO).
pub async fn generate(
    experiments: Arc<RwLock<Vec<ExperimentDocument>>>,
    batch_size: usize,
    tx: Sender<Arc<Vec<usize>>>,
) {
    let experiments = experiments.read().await;
    let batch: Vec<_> = (0..batch_size)
        .map(|_| {
            let mut rng = rand::thread_rng();
            let idx = rng.gen_range(0..experiments.len());
            let query_type = rng.gen_range(0..2);
            let experiment = &experiments[idx];
            query_type
        })
        .collect();
    tx.broadcast(Arc::new(batch))
        .await
        .expect("Receiver available");
}
