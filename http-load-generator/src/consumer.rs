use apache_avro::{from_value, Reader};
use rdkafka::{
    client::ClientContext,
    config::ClientConfig,
    consumer::stream_consumer::StreamConsumer,
    consumer::{CommitMode, Consumer, ConsumerContext, Rebalance},
    message::Message,
};
use serde::Deserialize;
use std::{cmp::Ordering, sync::Arc};
use tokio::{
    sync::{mpsc::Sender, RwLock},
    time::{self, Duration},
};

struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {
    fn post_rebalance(&self, rebalance: &Rebalance) {
        println!("Post rebalance {:?}", rebalance);
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Measurement {
    pub timestamp: f64,
    pub temperature: f32,
}

impl PartialEq for Measurement {
    fn eq(&self, other: &Self) -> bool {
        let timestamp_diff = (other.timestamp - self.timestamp).powi(2);
        let temperature_diff = (other.temperature - self.temperature).powi(2);
        (timestamp_diff < 0.000001) && (temperature_diff < 0.000001)
    }
}

impl PartialOrd for Measurement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct TempRange {
    pub upper_threshold: f32,
    pub lower_threshold: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExperimentDocumentData {
    pub experiment: String,
    pub measurements: Vec<Measurement>,
    pub temperature_range: TempRange,
}

#[derive(Debug, Clone)]
pub struct ExperimentDocument {
    pub experiment: String,
    pub measurements: Vec<Measurement>,
    pub temperature_range: TempRange,
    out_of_bounds: Option<Vec<Measurement>>,
}

impl From<ExperimentDocumentData> for ExperimentDocument {
    fn from(mut data: ExperimentDocumentData) -> Self {
        data.measurements.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Self {
            experiment: data.experiment,
            measurements: data.measurements,
            temperature_range: data.temperature_range,
            out_of_bounds: None,
        }
    }
}

impl ExperimentDocument {
    pub fn compute_out_of_bounds(&self) -> Vec<Measurement> {
        let TempRange {
            upper_threshold,
            lower_threshold,
        } = self.temperature_range;
        self.measurements
            .iter()
            .filter(|measurement| {
                let temperature = measurement.temperature;
                (temperature > upper_threshold) || (temperature < lower_threshold)
            })
            .map(|measurement| measurement.clone())
            .collect()
    }

    pub fn get_cached_out_of_bounds(&self) -> Option<&[Measurement]> {
        self.out_of_bounds
            .as_ref()
            .map(|measurements| &measurements[..])
    }

    pub fn set_out_of_bounds(&mut self, measurements: Vec<Measurement>) {
        self.out_of_bounds = Some(measurements);
    }

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

async fn read_loop<T>(consumer: StreamConsumer<T>, tx: Sender<ExperimentDocument>)
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
                        from_value::<ExperimentDocumentData>(&value.unwrap())
                            .expect("Received invalid event")
                            .into();
                    experiment_document
                        .measurements
                        .sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        // TODO: Parameterized sleep
                        time::sleep(Duration::from_millis(5 * 1000)).await;
                        tx.send(experiment_document)
                            .await
                            .expect("Receiver available");
                    });
                }
                consumer.commit_message(&b, CommitMode::Async).unwrap();
            }
        };
    }
}

pub async fn start(brokers: &str, group_id: &str, topics: &[&str], tx: Sender<ExperimentDocument>) {
    let context = CustomContext;

    let consumer: StreamConsumer<CustomContext> = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
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

    read_loop(consumer, tx).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_vec() {
        let mut v1 = vec![
            Measurement {
                timestamp: 0.03,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.00,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.02,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.01,
                temperature: 20.0,
            },
        ];
        v1.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let v2 = vec![
            Measurement {
                timestamp: 0.00,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.01,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.02,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.03,
                temperature: 20.0,
            },
        ];
        assert!(v1 == v2)
    }

    #[test]
    fn compare_measurements_with_different_precision() {
        let mut v1 = vec![
            Measurement {
                timestamp: 0.0031,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.0001,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.0021,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.0011,
                temperature: 20.0,
            },
        ];
        v1.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let v2 = vec![
            Measurement {
                timestamp: 0.000,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.001,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.002,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.003,
                temperature: 20.0,
            },
        ];
        assert!(v1 == v2)
    }

    #[test]
    fn get_experiment_document_slice() {
        let e1: ExperimentDocument = ExperimentDocumentData {
            experiment: "1234".into(),
            measurements: vec![
                Measurement {
                    timestamp: 0.0031,
                    temperature: 20.0,
                },
                Measurement {
                    timestamp: 0.0001,
                    temperature: 20.0,
                },
                Measurement {
                    timestamp: 0.0021,
                    temperature: 20.0,
                },
                Measurement {
                    timestamp: 0.0011,
                    temperature: 20.0,
                },
            ],
            temperature_range: TempRange {
                upper_threshold: 20.0,
                lower_threshold: 10.0,
            },
        }
        .into();
        let v1 = e1
            .get_measurements_slice(0.0, 5.0)
            .expect("Should not error");
        let v2 = &[
            Measurement {
                timestamp: 0.000,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.001,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.002,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.003,
                temperature: 20.0,
            },
        ];
        assert!(v1 == v2);

        let v1 = e1
            .get_measurements_slice(0.001, 5.0)
            .expect("Should not error");
        let v2 = &[
            Measurement {
                timestamp: 0.001,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.002,
                temperature: 20.0,
            },
            Measurement {
                timestamp: 0.003,
                temperature: 20.0,
            },
        ];
        assert!(v1 == v2);
    }
}
