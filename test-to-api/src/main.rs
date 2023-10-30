use clap::{command, Arg, ArgAction};
use dashmap::DashMap;
use futures::future;
use poem::{
    listener::TcpListener,
    web::{Data, Query},
    EndpointExt, Route, Server,
};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService};
use rand::Rng;
use serde::Deserialize;
use std::sync::Arc;
use tokio::time::{self, Duration};

mod consumer;

use crate::consumer::{ExperimentDocument, Measurement};

#[derive(Deserialize, Debug)]
struct TemperatureQueryParams {
    #[serde(rename = "experiment-id")]
    experiment_id: String,
    #[serde(rename = "start-time")]
    start_time: f64,
    #[serde(rename = "end-time")]
    end_time: f64,
}

#[derive(Deserialize, Debug)]
struct OutOfBoundsQueryParams {
    #[serde(rename = "experiment-id")]
    experiment_id: String,
}

struct Api;

#[OpenApi]
/// Hello world
impl Api {
    #[oai(path = "/temperature", method = "get")]
    async fn temperature(
        &self,
        map: Data<&Arc<DashMap<String, ExperimentDocument>>>,
        params: Query<TemperatureQueryParams>,
    ) -> PlainText<String> {
        time::sleep(Duration::from_millis(15)).await;
        let map = map.0;
        let TemperatureQueryParams {
            experiment_id,
            start_time,
            end_time,
        } = params.0;
        let experiment = map
            .get(&experiment_id)
            .expect(&format!("Experiment `{:?}` does not exist", experiment_id));
        let measurements = experiment
            .get_measurements_slice(start_time, end_time)
            .unwrap();

        let random_value = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..1.0)
        };
        if random_value < 0.1 {
            PlainText(serde_json::to_string(&experiment.measurements).unwrap())
        } else if random_value < 0.2 {
            PlainText(String::from("invalid serialized data"))
        } else {
            PlainText(serde_json::to_string(&measurements).unwrap())
        }
    }

    #[oai(path = "/temperature/out-of-range", method = "get")]
    async fn out_of_bounds(
        &self,
        map: Data<&Arc<DashMap<String, ExperimentDocument>>>,
        params: Query<OutOfBoundsQueryParams>,
    ) -> PlainText<String> {
        time::sleep(Duration::from_millis(15)).await;
        let OutOfBoundsQueryParams { experiment_id } = params.0;
        let experiment = map
            .get(&experiment_id)
            .expect(&format!("Experiment `{:?}` does not exist", experiment_id));
        let measurements = &experiment.measurements;
        let upper_threshold = experiment.temperature_range.upper_threshold;
        let lower_threshold = experiment.temperature_range.lower_threshold;
        let measurements: Vec<Measurement> = measurements
            .iter()
            .filter(|measurement| {
                let temperature = measurement.temperature;
                (temperature > upper_threshold) || (temperature < lower_threshold)
            })
            .map(|measurement| measurement.clone())
            .collect();

        let random_value = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..1.0)
        };
        if random_value < 0.1 {
            PlainText(serde_json::to_string(&experiment.measurements).unwrap())
        } else if random_value < 0.2 {
            PlainText(String::from("invalid serialized data"))
        } else {
            PlainText(serde_json::to_string(&measurements).unwrap())
        }
    }
}

#[tokio::main]
async fn main() {
    let mut matches = command!() // requires `cargo` feature
        .next_line_help(true)
        .arg(Arg::new("secret-key")
            .required(false)
            .long("secret-key")
            .action(ArgAction::Set)
            .default_value("QJUHsPhnA0eiqHuJqsPgzhDozYO4f1zh")
            .help("<key> is a 32 character string that must match the key being passed to the notifications-service")
        )
        .arg(Arg::new("broker-list")
            .required(true)
            .action(ArgAction::Set)
            .short('b')
            .long("brokers")
            .help("<broker-list> is a comma-seperated list of brokers. E.g.  For a single local broker `localhost:9092`. For multiple brokers `localhost:9092,localhost:9093`")
        )
        .arg(Arg::new("topic")
            .required(true)
            .long("topic")
            .default_value("experiment")
            .action(ArgAction::Set)
        )
        .arg(Arg::new("group-id")
            .required(true)
            .long("group-id")
            .action(ArgAction::Set)
        )
        .get_matches();

    let api_service =
        OpenApiService::new(Api, "Hello World", "1.0").server("http://localhost:3000");
    let ui = api_service.swagger_ui();

    let experiments: Arc<DashMap<String, ExperimentDocument>> = Arc::new(DashMap::new());
    let app = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .data(experiments.clone());

    let mut handles = vec![];

    handles.push(tokio::spawn(async move {
        consumer::start(
            &matches
                .remove_one::<String>("broker-list")
                .expect("required"),
            &matches.remove_one::<String>("group-id").expect("required"),
            &[&matches.remove_one::<String>("topic").expect("required")],
            experiments,
        )
        .await
    }));
    handles.push(tokio::spawn(async move {
        Server::new(TcpListener::bind("0.0.0.0:3003"))
            .run(app)
            .await
            .unwrap()
    }));
    future::join_all(handles).await;
}
