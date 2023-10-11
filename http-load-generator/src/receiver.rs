use async_broadcast::broadcast;
use futures::future;
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, RwLock};

use crate::consumer::ExperimentDocument;
use crate::generator;

async fn receive_experiments(
    experiments: Arc<RwLock<Vec<ExperimentDocument>>>,
    rx: &mut Receiver<ExperimentDocument>,
) {
    let experiment = rx.recv().await.expect("Sender available");
    let mut experiments = experiments.write().await;
    experiments.push(experiment);
    while let Ok(experiment) = rx.try_recv() {
        experiments.push(experiment);
    }
}

pub async fn start(mut rx: Receiver<ExperimentDocument>) {
    let experiments: Arc<RwLock<Vec<ExperimentDocument>>> = Arc::new(RwLock::new(vec![]));
    let (s, r) = broadcast::<Arc<Vec<usize>>>(1000);
    loop {
        receive_experiments(experiments.clone(), &mut rx).await;
        let mut handles: Vec<_> = (0..6)
            .map(|_| {
                let experiments = experiments.clone();
                let s = s.clone();
                tokio::spawn(async move { generator::generate(experiments.clone(), 10, s).await })
            })
            .collect();

        handles.append(
            &mut (0..2)
                .map(|idx| {
                    let mut r = r.clone();
                    tokio::spawn(async move {
                        while let Ok(load) = r.recv().await {
                            println!("{} - {:?}", idx, load);
                        }
                    })
                })
                .collect(),
        );

        future::join_all(handles).await;
    }
}
