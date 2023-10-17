use actix_web::{
    get,
    web::{self, Data},
    App, HttpResponse, HttpServer, Responder, Result,
};
use prometheus_client::{
    encoding::{text, EncodeLabelSet, EncodeLabelValue},
    metrics::{counter::Counter, family::Family},
    registry::Registry,
};
use std::sync::Mutex;

use crate::requests::ResponseError;

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
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            response_count: Family::<ResponseCountLabels, Counter>::default(),
        }
    }
}

pub struct MetricServer {
    metrics: Metrics,
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

        Self { metrics, registry }
    }

    pub fn start(self) {
        let state = Data::new(Mutex::new(self.registry));
        let server = HttpServer::new(move || {
            App::new().service(get_metrics).app_data(state.clone())
        })
            .bind(("127.0.0.1", 8080))
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
