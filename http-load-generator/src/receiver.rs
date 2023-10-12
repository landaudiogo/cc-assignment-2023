use async_broadcast::broadcast;
use futures::future;
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, RwLock};

use crate::consumer::ExperimentDocument;
use crate::generator::{self, APIQuery};

async fn receive_experiments(
    experiments: Arc<RwLock<Vec<Arc<ExperimentDocument>>>>,
    rx: &mut Receiver<Arc<ExperimentDocument>>,
) {
    let experiment = rx.recv().await.expect("Sender available");
    let mut experiments = experiments.write().await;
    experiments.push(experiment);
    while let Ok(experiment) = rx.try_recv() {
        experiments.push(experiment);
    }
}

pub async fn start(mut rx: Receiver<Arc<ExperimentDocument>>) {
    let experiments = Arc::new(RwLock::new(vec![]));
    let (s, r) = broadcast::<Arc<Vec<APIQuery>>>(1000);

    // Spawn 1 thread per group
    // TODO: Parametrized list of group IP's
    for idx in 0..2 {
        let mut r = r.clone();
        tokio::spawn(async move {
            while let Ok(load) = r.recv().await {
                for query in load.iter() {
                    if idx == 1 {
                        continue
                    }
                    match query {
                        APIQuery::Temperature { experiment, start_time, end_time } => {
                            println!("{}, {}, {}", experiment.experiment, start_time, end_time);
                        }, 
                        APIQuery::OutOfBounds { experiment } => {
                            println!("{}", experiment.experiment);
                        }
                    }
                } 
            }
        });
    }

    // Each iteration receives messages and generates load for the next minute
    loop {
        receive_experiments(experiments.clone(), &mut rx).await;
        let handles: Vec<_> = (0..6)
            .map(|_| {
                let experiments = experiments.clone();
                let s = s.clone();
                tokio::spawn(async move { generator::generate(experiments.clone(), 10, s).await })
            })
            .collect();

        future::join_all(handles).await;
    }
}
