use async_broadcast::Sender;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::consumer::ExperimentDocument;


#[derive(Debug)]
pub enum APIQuery {
    OutOfBounds {
        experiment: Arc<ExperimentDocument>,
    }, 
    Temperature {
        experiment: Arc<ExperimentDocument>,
        start_time: f64,
        end_time: f64,
    }
}

fn generate_temperature_query(experiment: Arc<ExperimentDocument>) -> APIQuery {
    let mut rng = rand::thread_rng();
    let mut idxs = vec![
        rng.gen_range(0..experiment.measurements.len()), 
        rng.gen_range(0..experiment.measurements.len())
    ];
    idxs.sort();
    let start_time = experiment.measurements[idxs[0]].timestamp;
    let end_time = experiment.measurements[idxs[1]].timestamp;
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
    experiments: Arc<RwLock<Vec<Arc<ExperimentDocument>>>>,
    batch_size: usize,
    tx: Sender<Arc<Vec<APIQuery>>>,
) {
    let experiments = experiments.read().await;
    let batch: Vec<_> = (0..batch_size)
        .map(|_| {
            let mut rng = rand::thread_rng();
            let idx = rng.gen_range(0..experiments.len());
            let experiment = experiments[idx].clone();
            let query_type = rng.gen_range(0..2);
            match query_type {
                0 => APIQuery::OutOfBounds { experiment }, 
                1 => generate_temperature_query(experiments[idx].clone()),
                _ => panic!("There are only 2 query types"),
            }
        })
        .collect();
    tx.broadcast(Arc::new(batch))
        .await
        .expect("Receiver available");
}
