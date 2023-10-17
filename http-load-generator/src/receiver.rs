use async_broadcast::broadcast;
use futures::future;
use rand::Rng;
use std::sync::Arc;
use tokio::{
    sync::{mpsc::Receiver, RwLock},
    time::{self, Duration},
};

use crate::experiment::ExperimentDocument;
use crate::generator::{self, APIQuery};
use crate::metric::{MetricServer, Metrics};
use crate::requests::{Host, Requestor};

struct ExperimentReceiverConfig {}

async fn receive_experiments(
    experiments: Arc<RwLock<Vec<Arc<RwLock<ExperimentDocument>>>>>,
    rx: &mut Receiver<ExperimentDocument>,
) {
    let mut experiments = experiments.write().await;
    while let Ok(experiment) = rx.try_recv() {
        experiments.push(Arc::new(RwLock::new(experiment)));
    }
}

pub async fn start(mut rx: Receiver<ExperimentDocument>) {
    let experiments = Arc::new(RwLock::new(vec![]));
    let (batch_tx, batch_rx) = broadcast::<Arc<Vec<APIQuery>>>(1000);

    // Create a sample counter metric family utilizing the above custom label
    // type, representing the number of HTTP requests received.
    let metrics = Metrics::new();
    let metric_server = MetricServer::new(metrics.clone());
    metric_server.start();

    let experiment = rx.recv().await.expect("Sender available");
    experiments
        .write()
        .await
        .push(Arc::new(RwLock::new(experiment)));

    // Spawn 1 thread per group
    // TODO: Parametrized list of group IP's
    // TODO: Parametrized start wait
    let servers = [Host::new("landau", "http://localhost:3000")];
    for host in servers {
        let batch_rx = batch_rx.clone();
        let mut requestor = Requestor::new(host, batch_rx, metrics.clone());

        tokio::spawn(async move {
            time::sleep(Duration::from_millis(5000)).await;
            requestor.start().await;
        });
    }

    // Each iteration receives messages and generates load for the next minute
    // Sleep for 60 seconds after generating all the requests, if there are new experiments, read
    // them and generate the load, otherwise just generate the load with the experiments available.
    //
    // TODO: Parametrizable MAX_BATCH_SIZE
    loop {
        receive_experiments(experiments.clone(), &mut rx).await;
        let mut handles = vec![];
        handles.push(tokio::spawn(async move {
            time::sleep(Duration::from_millis(60 * 1000)).await;
        }));
        let batch_size = {
            let mut rng = rand::thread_rng();
            rng.gen_range(100..200)
        };
        handles.append(
            &mut (0..60)
                .map(|_| {
                    let experiments = experiments.clone();
                    let batch_tx = batch_tx.clone();
                    tokio::spawn(async move {
                        generator::generate(experiments.clone(), batch_size, batch_tx).await
                    })
                })
                .collect(),
        );
        future::join_all(handles).await;
    }
}
