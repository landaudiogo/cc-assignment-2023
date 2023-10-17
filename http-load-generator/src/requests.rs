use async_broadcast::Receiver;
use futures::{stream, StreamExt};
use prometheus_client::metrics::{counter::Counter, family::Family};
use reqwest::{Client, Error, RequestBuilder, Response};
use std::sync::Arc;
use tokio::{
    sync::RwLock,
    time::{self, Duration},
};

use crate::generator::APIQuery;
use crate::metric::Metrics;
use crate::{
    consumer::{ExperimentDocument, Measurement},
    metric::{ResponseCountLabels, ResponseType},
};

#[derive(Debug)]
pub enum ResponseError {
    ServerError,
    BodyDecodingError,
    DeserializationError,
    ValidationError,
}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Server response error")
    }
}

impl std::error::Error for ResponseError {}

pub struct Host {
    host_name: String,
    base_url: String,
}

impl Host {
    pub fn new(host_name: &str, base_url: &str) -> Self {
        Self {
            host_name: host_name.into(),
            base_url: base_url.into(),
        }
    }
}

pub struct Requestor {
    host: Host,
    batch_rx: Receiver<Arc<Vec<APIQuery>>>,
    client: Client,
    metrics: Metrics,
}

impl Requestor {
    pub fn new(host: Host, batch_rx: Receiver<Arc<Vec<APIQuery>>>, metrics: Metrics) -> Self {
        Self {
            host,
            batch_rx,
            client: Client::new(),
            metrics,
        }
    }

    async fn prepare_temperature_request(
        &self,
        experiment: Arc<RwLock<ExperimentDocument>>,
        start_time: f64,
        end_time: f64,
    ) -> RequestBuilder {
        self.client
            .get(format!("{}/temperature", self.host.base_url))
            .query(&[
                ("experiment-id", experiment.read().await.experiment.as_str()),
                ("start-time", format!("{}", start_time).as_str()),
                ("end-time", format!("{}", end_time).as_str()),
            ])
    }

    async fn validate_temperature_response(
        &self,
        response: Result<Response, Error>,
        experiment: Arc<RwLock<ExperimentDocument>>,
        start_time: f64,
        end_time: f64,
    ) -> Result<(), ResponseError> {
        let response = match response {
            Ok(response) => response,
            Err(_) => {
                return Err(ResponseError::ServerError);
            }
        };

        let content = match response.text().await {
            Ok(content) => content,
            Err(_) => return Err(ResponseError::BodyDecodingError),
        };

        let mut measurements: Vec<Measurement> = match serde_json::from_str(&content) {
            Ok(measurements) => measurements,
            Err(_) => return Err(ResponseError::DeserializationError),
        };
        measurements.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let experiment_read = experiment.read().await;
        let measurements_cmp = experiment_read
            .get_measurements_slice(start_time, end_time)
            .expect("Valid start and end times");

        if measurements_cmp != measurements {
            Err(ResponseError::ValidationError)
        } else {
            Ok(())
        }
    }

    async fn make_temperature_request(
        &self,
        experiment: Arc<RwLock<ExperimentDocument>>,
        start_time: f64,
        end_time: f64,
    ) -> Result<(), ResponseError> {
        let request = self
            .prepare_temperature_request(experiment.clone(), start_time, end_time)
            .await;
        let response = tokio::spawn(async move { request.send().await })
            .await
            .expect("Join should not fail");

        self.validate_temperature_response(response, experiment, start_time, end_time)
            .await
    }

    async fn prepare_out_of_bounds_request(
        &self,
        experiment: Arc<RwLock<ExperimentDocument>>,
    ) -> RequestBuilder {
        self.client
            .get(format!("{}/temperature/out-of-bounds", self.host.base_url))
            .query(&[("experiment-id", experiment.read().await.experiment.as_str())])
    }

    async fn validate_out_of_bounds_response(
        &self,
        response: Result<Response, Error>,
        experiment: Arc<RwLock<ExperimentDocument>>,
    ) -> Result<(), ResponseError> {
        let response = match response {
            Ok(response) => response,
            Err(_) => return Err(ResponseError::ServerError),
        };

        let content = match response.text().await {
            Ok(content) => content,
            Err(_) => return Err(ResponseError::BodyDecodingError),
        };

        let mut measurements: Vec<Measurement> = match serde_json::from_str(&content) {
            Ok(measurements) => measurements,
            Err(_) => return Err(ResponseError::DeserializationError),
        };
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
        if measurements_cmp != measurements {
            Err(ResponseError::ValidationError)
        } else {
            Ok(())
        }
    }

    async fn make_out_of_bounds_request(
        &self,
        experiment: Arc<RwLock<ExperimentDocument>>,
    ) -> Result<(), ResponseError> {
        let request = self.prepare_out_of_bounds_request(experiment.clone()).await;
        let response = tokio::spawn(async move { request.send().await })
            .await
            .expect("Join should not fail");

        self.validate_out_of_bounds_response(response, experiment)
            .await
    }

    async fn process_batch(&mut self, batch: Arc<Vec<APIQuery>>) {
        let sleep_handle = tokio::spawn(time::sleep(Duration::from_millis(1000)));
        stream::iter(batch.iter())
            .map(|query| async {
                for _ in 0..3 {
                    let query = query.clone();
                    match query {
                        APIQuery::Temperature {
                            experiment,
                            start_time,
                            end_time,
                        } => {
                            let res = self
                                .make_temperature_request(experiment.clone(), start_time, end_time)
                                .await;

                            self.metrics
                                .response_count
                                .get_or_create(&ResponseCountLabels {
                                    host_name: self.host.host_name.clone(),
                                    endpoint: "/temperature".to_string(),
                                    response_type: ResponseType::from(&res),
                                })
                                .inc();
                            if let Err(ResponseError::ServerError) = res {
                                continue;
                            } else {
                                return Some(());
                            }
                        }
                        APIQuery::OutOfBounds { experiment } => {
                            let res = self.make_out_of_bounds_request(experiment.clone()).await;
                            self.metrics
                                .response_count
                                .get_or_create(&ResponseCountLabels {
                                    host_name: self.host.host_name.clone(),
                                    endpoint: "/temperature/out-of-bounds".to_string(),
                                    response_type: ResponseType::from(&res),
                                })
                                .inc();
                            if let Err(ResponseError::ServerError) = res {
                                continue;
                            } else {
                                return Some(());
                            }
                        }
                    }
                }
                return None;
            })
            .boxed()
            .buffer_unordered(50)
            .for_each(|response| async move {
                match response {
                    None => {
                        // Register failure in histogram
                    }
                    _ => {}
                }
            })
            .await;

        sleep_handle.await.expect("Should not fail");
        println!("Performed {} requests", batch.len());
    }

    pub async fn start(&mut self) {
        while let Ok(batch) = self.batch_rx.recv().await {
            self.process_batch(batch).await;
        }
    }
}
