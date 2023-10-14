use async_broadcast::broadcast;
use futures::future;
use rand::Rng;
use std::sync::Arc;
use tokio::{
    sync::{mpsc::Receiver, RwLock},
    time::{self, Duration},
};

use crate::consumer::ExperimentDocument;
use crate::generator::{self, APIQuery};
use crate::requests;

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
    let (s, r) = broadcast::<Arc<Vec<APIQuery>>>(1000);

    let experiment = rx.recv().await.expect("Sender available");
    experiments
        .write()
        .await
        .push(Arc::new(RwLock::new(experiment)));

    // Spawn 1 thread per group
    // TODO: Parametrized list of group IP's
    // TODO: Parametrized start wait
    let servers: Vec<String> = vec!["http://localhost:3000".into()];
    for base_url in servers {
        let r = r.clone();
        let base_url = base_url.clone();
        tokio::spawn(async move {
            time::sleep(Duration::from_millis(5000)).await;
            requests::traverse_load(base_url, r).await;
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
                    let s = s.clone();
                    tokio::spawn(async move {
                        generator::generate(experiments.clone(), batch_size, s).await
                    })
                })
                .collect(),
        );
        future::join_all(handles).await;
    }
}
