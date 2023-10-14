use async_broadcast::Sender;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::consumer::ExperimentDocument;

#[derive(Debug, Clone)]
pub enum APIQuery {
    OutOfBounds {
        experiment: Arc<RwLock<ExperimentDocument>>,
    },
    Temperature {
        experiment: Arc<RwLock<ExperimentDocument>>,
        start_time: f64,
        end_time: f64,
    },
}

async fn generate_temperature_query(experiment: Arc<RwLock<ExperimentDocument>>) -> APIQuery {
    let num_experiments = experiment.read().await.measurements.len();
    let mut idxs = {
        let mut rng = rand::thread_rng();
        vec![
            rng.gen_range(0..num_experiments),
            rng.gen_range(0..num_experiments),
        ]
    };
    idxs.sort();
    let start_time =
        (experiment.read().await.measurements[idxs[0]].timestamp * 1000.0).floor() / 1000.0;
    let end_time =
        (experiment.read().await.measurements[idxs[1]].timestamp * 1000.0).ceil() / 1000.0;

    APIQuery::Temperature {
        experiment,
        start_time,
        end_time,
    }
}

/// Generate 60 request batches
///
/// The batches are stored on a shared queue, wherein the receiver appends to the queue, and the
/// executors pop from the queue (FIFO).
pub async fn generate(
    experiments: Arc<RwLock<Vec<Arc<RwLock<ExperimentDocument>>>>>,
    batch_size: usize,
    tx: Sender<Arc<Vec<APIQuery>>>,
) {
    let experiments = experiments.read().await;
    let mut batch = vec![];
    for _ in 0..batch_size {
        let idx = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..experiments.len())
        };
        let experiment = experiments[idx].clone();
        let query_type = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..2)
        };
        let query = match query_type {
            0 => APIQuery::OutOfBounds { experiment },
            1 => generate_temperature_query(experiments[idx].clone()).await,
            _ => panic!("There are only 2 query types"),
        };
        batch.push(query);
    }
    tx.broadcast(Arc::new(batch))
        .await
        .expect("Receiver available");
}
