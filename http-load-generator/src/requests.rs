use async_broadcast::Receiver;
use futures::future;
use reqwest::Client;
use std::sync::Arc;
use tokio::{
    sync::RwLock,
    time::{self, Duration},
};

use crate::consumer::{ExperimentDocument, Measurement};
use crate::generator::APIQuery;

// TODO: Parametrizable url
async fn make_out_of_bounds_request(
    client: Client,
    base_url: String,
    experiment: Arc<RwLock<ExperimentDocument>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let res = client
        .get(format!("{base_url}/temperature/out-of-bounds"))
        .query(&[("experiment-id", experiment.read().await.experiment.as_str())])
        .send()
        .await
        .unwrap();

    let mut measurements: Vec<Measurement> =
        serde_json::from_str(&res.text_with_charset("utf-8").await.unwrap()).unwrap();
    measurements.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let is_none = experiment.read().await.get_cached_out_of_bounds().is_none();
    if is_none {
        let mut experiment = experiment.write().await;
        if experiment.get_cached_out_of_bounds().is_none() {
            let measurements = experiment.compute_out_of_bounds();
            experiment.set_out_of_bounds(measurements);
        }
    }
    let experiment_read = experiment.read().await;
    let measurements_cmp = experiment_read.get_cached_out_of_bounds().unwrap();
    // println!("out-of-bounds {}", &measurements[..] == measurements_cmp);
    Ok(())
}

async fn make_temperature_request(
    client: Client,
    base_url: String,
    experiment: Arc<RwLock<ExperimentDocument>>,
    start_time: f64,
    end_time: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let res = client
        .get(format!("{base_url}/temperature"))
        .query(&[
            ("experiment-id", experiment.read().await.experiment.as_str()),
            ("start-time", format!("{}", start_time).as_str()),
            ("end-time", format!("{}", end_time).as_str()),
        ])
        .send()
        .await
        .unwrap();

    let mut measurements: Vec<Measurement> =
        serde_json::from_str(&res.text_with_charset("utf-8").await.unwrap()).unwrap();
    measurements.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let experiment_read = experiment.read().await;
    let measurements_cmp = experiment_read
        .get_measurements_slice(start_time, end_time)
        .unwrap();
    // println!("temperature {}", &measurements[..] == measurements_cmp);
    Ok(())
}

pub async fn traverse_load(base_url: String, mut r: Receiver<Arc<Vec<APIQuery>>>) {
    while let Ok(load) = r.recv().await {
        let client = reqwest::Client::new();
        let mut handles = vec![];
        handles.push(tokio::spawn(time::sleep(Duration::from_millis(1000))));
        for query in load.iter() {
            let query = query.clone();
            match query {
                APIQuery::Temperature {
                    experiment,
                    start_time,
                    end_time,
                } => {
                    let client = client.clone();
                    let base_url = base_url.clone();
                    handles.push(tokio::spawn(async move {
                        make_temperature_request(
                            client, base_url, experiment, start_time, end_time,
                        )
                        .await;
                    }));
                }
                APIQuery::OutOfBounds { experiment } => {
                    let client = client.clone();
                    let base_url = base_url.clone();
                    handles.push(tokio::spawn(async move {
                        make_out_of_bounds_request(client, base_url, experiment).await;
                    }));
                }
            }
        }
        future::join_all(handles).await;
        println!("Performed {} requests", load.len());
    }
}
