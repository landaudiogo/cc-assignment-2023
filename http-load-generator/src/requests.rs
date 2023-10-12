use async_broadcast::Receiver;
use futures::future;
use reqwest::Client;
use std::sync::Arc;
use tokio::time::{self, Duration};

use crate::consumer::{ExperimentDocument, Measurement};
use crate::generator::APIQuery;

// TODO: Parametrizable url
async fn make_out_of_bounds_request(
    client: Client,
    base_url: String,
    experiment: Arc<ExperimentDocument>,
) -> Result<(), Box<dyn std::error::Error>> {
    let res = client
        .get(format!("{base_url}/temperature/out-of-bounds"))
        .query(&[("experiment-id", experiment.experiment.clone())])
        .send()
        .await
        .unwrap();

    let measurements: Vec<Measurement> =
        serde_json::from_str(&res.text_with_charset("utf-8").await.unwrap()).unwrap();
    Ok(())
}

async fn make_temperature_request(
    client: Client,
    base_url: String,
    experiment: Arc<ExperimentDocument>,
    start_time: f64,
    end_time: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let res = client
        .get(format!("{base_url}/temperature"))
        .query(&[
            ("experiment-id", experiment.experiment.clone()),
            ("start-time", format!("{}", start_time)),
            ("end-time", format!("{}", end_time)),
        ])
        .send()
        .await
        .unwrap();

    let measurements: Vec<Measurement> =
        serde_json::from_str(&res.text_with_charset("utf-8").await.unwrap()).unwrap();
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
