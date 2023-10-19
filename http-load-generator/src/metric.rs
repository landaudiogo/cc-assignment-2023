use actix_web::{get, web::Data, App, HttpServer, Responder, Result};
use prometheus_client::{
    encoding::{text, EncodeLabelSet, EncodeLabelValue},
    metrics::{counter::Counter, family::Family, gauge::Gauge, histogram::Histogram},
    registry::Registry,
};
use std::sync::Mutex;

use crate::request::ResponseError;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RequestRateLabels {
    pub host_name: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ResponseCountLabels {
    pub host_name: String,
    pub response_type: ResponseType,
    pub endpoint: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum ResponseType {
    Ok,
    ServerError,
    BodyDecodingError,
    DeserializationError,
    ValidationError,
}

impl From<&Result<(), ResponseError>> for ResponseType {
    fn from(response: &Result<(), ResponseError>) -> Self {
        match response {
            Ok(()) => ResponseType::Ok,
            Err(ResponseError::ServerError) => ResponseType::ServerError,
            Err(ResponseError::BodyDecodingError) => ResponseType::BodyDecodingError,
            Err(ResponseError::DeserializationError) => ResponseType::DeserializationError,
            Err(ResponseError::ValidationError) => ResponseType::ValidationError,
        }
    }
}

#[derive(Clone)]
pub struct Metrics {
    pub response_count: Family<ResponseCountLabels, Counter>,
    pub response_time_histogram: Family<ResponseCountLabels, Histogram>,
    pub target_request_rate: Family<RequestRateLabels, Gauge>,
    pub effective_request_rate: Family<RequestRateLabels, Gauge>,
}

impl Metrics {
    pub fn new() -> Self {
        let response_time_histogram =
            Family::<ResponseCountLabels, Histogram>::new_with_constructor(|| {
                let custom_buckets = [
                    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                ];
                Histogram::new(custom_buckets.into_iter())
            });
        Self {
            response_count: Family::<ResponseCountLabels, Counter>::default(),
            response_time_histogram,
            target_request_rate: Family::<RequestRateLabels, Gauge>::default(),
            effective_request_rate: Family::<RequestRateLabels, Gauge>::default(),
        }
    }
}

pub struct MetricServer {
    registry: Registry,
}

impl MetricServer {
    pub fn new(metrics: Metrics) -> Self {
        let mut registry = <Registry>::default();
        registry.register(
            "response_count",
            "Count of response",
            metrics.response_count.clone(),
        );
        registry.register(
            "response_rtt_histogram",
            "Response round trip time in seconds",
            metrics.response_time_histogram.clone(),
        );
        registry.register(
            "target_request_rate",
            "The target request rate at which a host is being queried",
            metrics.target_request_rate.clone(),
        );
        registry.register(
            "effective_request_rate",
            "Effective request rate at which the host is being queried",
            metrics.effective_request_rate.clone(),
        );

        Self { registry }
    }

    pub fn start(self) {
        let state = Data::new(Mutex::new(self.registry));
        let server =
            HttpServer::new(move || App::new().service(get_metrics).app_data(state.clone()))
                .bind(("0.0.0.0", 3002))
                .unwrap()
                .run();
        tokio::spawn(server);
    }
}

#[get("/metrics")]
async fn get_metrics(state: Data<Mutex<Registry>>) -> impl Responder {
    let state = state.lock().unwrap();
    let mut body = String::new();
    text::encode(&mut body, &state).unwrap();
    body
}
