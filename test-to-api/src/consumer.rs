use apache_avro::{from_value, Reader};
use dashmap::DashMap;
use rdkafka::{
    client::ClientContext,
    config::ClientConfig,
    consumer::stream_consumer::StreamConsumer,
    consumer::{CommitMode, Consumer, ConsumerContext, Rebalance},
    message::Message,
};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, sync::Arc};

struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {
    fn post_rebalance(&self, rebalance: &Rebalance) {
        println!("Post rebalance {:?}", rebalance);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Measurement {
    pub timestamp: f64,
    pub temperature: f32,
}

impl PartialEq for Measurement {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl PartialOrd for Measurement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TempRange {
    pub upper_threshold: f32,
    pub lower_threshold: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExperimentDocument {
    pub experiment: String,
    pub measurements: Vec<Measurement>,
    pub temperature_range: TempRange,
}

impl ExperimentDocument {
    fn get_measurement_index_le(&self, timestamp: f64) -> Option<usize> {
        let len = self.measurements.len();
        let mut valid_range = [0, len - 1];
        let mut idx = len / 2;
        loop {
            let curr = &self.measurements[idx];
            match curr.timestamp.partial_cmp(&timestamp).unwrap() {
                Ordering::Less => {
                    if valid_range[0] == valid_range[1] {
                        return Some(idx);
                    }
                    valid_range[0] = valid_range[1] - (valid_range[1] - valid_range[0]) / 2;
                }
                Ordering::Equal => {
                    return Some(idx);
                }
                Ordering::Greater => {
                    if valid_range[0] == valid_range[1] {
                        if idx > 0 {
                            return Some(idx - 1);
                        } else {
                            return None;
                        }
                    }
                    valid_range[1] = valid_range[0] + (valid_range[1] - valid_range[0]) / 2;
                }
            }
            idx = valid_range[0] + (valid_range[1] - valid_range[0]) / 2;
        }
    }

    fn get_measurement_index_ge(&self, timestamp: f64) -> Option<usize> {
        let len = self.measurements.len();
        let mut valid_range = [0, len - 1];
        let mut idx = len / 2;
        loop {
            let curr = &self.measurements[idx];
            match curr.timestamp.partial_cmp(&timestamp).unwrap() {
                Ordering::Less => {
                    if valid_range[0] == valid_range[1] {
                        if idx == self.measurements.len() - 1 {
                            return None;
                        } else {
                            return Some(idx + 1);
                        }
                    }
                    valid_range[0] = valid_range[1] - (valid_range[1] - valid_range[0]) / 2;
                }
                Ordering::Equal => {
                    return Some(idx);
                }
                Ordering::Greater => {
                    if valid_range[0] == valid_range[1] {
                        return Some(idx);
                    }
                    valid_range[1] = valid_range[0] + (valid_range[1] - valid_range[0]) / 2;
                }
            }
            idx = valid_range[0] + (valid_range[1] - valid_range[0]) / 2;
        }
    }

    pub fn get_measurements_slice(&self, start_time: f64, end_time: f64) -> Option<&[Measurement]> {
        let start = self.get_measurement_index_ge(start_time);
        let end = self.get_measurement_index_le(end_time);
        let (start, end) = match (start, end) {
            (None, _) => return None,
            (_, None) => return None,
            (_, _) => (start.unwrap(), end.unwrap()),
        };
        if start >= end {
            Some(&self.measurements[start..start])
        } else {
            Some(&self.measurements[start..end + 1])
        }
    }
}

async fn read_loop<T>(consumer: StreamConsumer<T>, map: Arc<DashMap<String, ExperimentDocument>>)
where
    T: ConsumerContext + ClientContext + 'static,
{
    loop {
        match consumer.recv().await {
            Err(e) => println!("Kafka error: {}", e),
            Ok(b) => {
                let m = b.detach();
                let reader = Reader::new(m.payload().unwrap()).unwrap();
                for value in reader {
                    let mut experiment_document: ExperimentDocument =
                        from_value(&value.unwrap()).expect("Received invalid event");
                    println!("Adding experiment `{}`", experiment_document.experiment);
                    experiment_document
                        .measurements
                        .sort_by(|a, b| a.partial_cmp(b).unwrap());
                    map.insert(experiment_document.experiment.clone(), experiment_document);
                }
                consumer.commit_message(&b, CommitMode::Async).unwrap();
            }
        };
    }
}

pub async fn start(
    brokers: &str,
    group_id: &str,
    topics: &[&str],
    map: Arc<DashMap<String, ExperimentDocument>>,
) {
    let context = CustomContext;

    let consumer: StreamConsumer<CustomContext> = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .set("security.protocol", "SSL")
        .set("ssl.ca.location", "http-load-generator/auth/ca.crt")
        .set(
            "ssl.keystore.location",
            "http-load-generator/auth/kafka.keystore.pkcs12",
        )
        .set("ssl.keystore.password", "cc2023")
        .create_with_context(context)
        .expect("Consumer creation failed");

    consumer
        .subscribe(&topics.to_vec())
        .expect("Can't subscribe to specified topics");

    read_loop(consumer, map).await;
}
