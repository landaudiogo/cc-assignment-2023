use apache_avro::{from_value, Reader};
use clap::ArgMatches;
use event_hash::{HashData, NotificationType};
use rdkafka::{
    client::ClientContext,
    config::ClientConfig,
    consumer::stream_consumer::StreamConsumer,
    consumer::{CommitMode, Consumer, ConsumerContext, Rebalance},
    message::{Headers, Message},
};
use reqwest::Client;
use rand::Rng;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::time::{self, Duration};

#[derive(Deserialize, Debug)]
struct SensorTemperatureMeasured {
    experiment: String,
    sensor: String,
    measurement_id: String,
    timestamp: f64,
    temperature: f32,
    measurement_hash: String,
}

struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {
    fn post_rebalance(&self, rebalance: &Rebalance) {
        println!("Post rebalance {:?}", rebalance);
    }
}

pub struct ConsumeConfiguration {
    secret_key: String,
    group_id: String,
    brokers: String,
    topic: String,
}

impl From<&mut ArgMatches> for ConsumeConfiguration {
    fn from(args: &mut ArgMatches) -> Self {
        let secret_key = args.remove_one::<String>("secret-key").expect("Required");
        let brokers = args.remove_one::<String>("broker-list").expect("Required");
        let group_id = args.remove_one::<String>("group-id").expect("Required");
        let topic = args.remove_one::<String>("topic").expect("Required");

        ConsumeConfiguration {
            secret_key,
            group_id,
            brokers,
            topic,
        }
    }
}

pub struct Consume {
    config: ConsumeConfiguration,
    consumer: StreamConsumer<CustomContext>,
    client: Client,
}

impl Consume {
    pub fn new(config: ConsumeConfiguration) -> Self {
        let context = CustomContext;
        let consumer: StreamConsumer<CustomContext> = ClientConfig::new()
            .set("group.id", config.group_id.as_str())
            .set("bootstrap.servers", config.brokers.as_str())
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("security.protocol", "SSL")
            .set("ssl.ca.location", "auth/ca.crt")
            .set(
                "ssl.keystore.location",
                "auth/kafka.keystore.pkcs12",
            )
            .set("ssl.keystore.password", "cc2023")
            .create_with_context(context)
            .expect("Consumer creation failed");

        let client = Client::new();

        Self {
            config,
            consumer,
            client,
        }
    }

    pub async fn start(&self) {
        self.consumer
            .subscribe(&[self.config.topic.as_str()])
            .expect("Can't subscribe to specified topics");
        self.read_loop().await;
    }

    async fn read_loop(&self) {
        loop {
            match self.consumer.recv().await {
                Err(e) => println!("Kafka error: {}", e),
                Ok(b) => {
                    let m = b.detach();
                    let headers = m.headers();
                    if let None = headers {
                        continue;
                    }
                    let headers = headers.unwrap();
                    let record_name =
                        String::from_utf8(headers.get(0).unwrap().1.to_vec()).expect("Valid utf-8");
                    if record_name != "sensor_temperature_measured" {
                        continue;
                    }
                    let reader = Reader::new(m.payload().unwrap()).unwrap();
                    for value in reader {
                        let sensor_measurement: SensorTemperatureMeasured =
                            from_value::<SensorTemperatureMeasured>(&value.unwrap())
                                .expect("Received invalid event")
                                .into();

                        let hash_data = HashData::decrypt(
                            self.config.secret_key.as_bytes(),
                            &sensor_measurement.measurement_hash,
                        )
                        .expect("Valid measurement_hash");
                        if hash_data.notification_type.is_none() {
                            break;
                        }
                        let mut map = HashMap::new();
                        map.insert("researcher", hash_data.researcher.as_str());
                        map.insert("measurement_id", hash_data.measurement_id.as_str());
                        map.insert("experiment_id", hash_data.experiment_id.as_str());
                        map.insert("cipher_data", &sensor_measurement.measurement_hash);
                        match hash_data.notification_type {
                            Some(NotificationType::OutOfRange) => {
                                let sleep_secs = {
                                    let mut rng = rand::thread_rng();
                                    rng.gen_range(0..5)
                                };
                                time::sleep(Duration::from_millis(sleep_secs*1000)).await;
                                map.insert("notification_type", "OutOfRange");
                                self.client
                                    .post("http://localhost:3000/api/notify")
                                    .query(&[("token", "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJleHAiOjE3MDQxMjk4MTgsInN1YiI6ImxhbmRhdSJ9.Gvdz0IuwRPd0BloUGS9vWbRY97ZAB-HC43Fora-CFV-sejq8aNr-WNDd0mOuUP1XvgDO7gaiGv9UkOtlRBEL_TlWizDmS2buTrKWCu2C6x8U1_NR5dfjF0sKdADA4DSLxxTwiuImXNHbtkhbZjbzpMn5CYlkuydbn2Rlg4lNAV91k2zWaM4op1IQO2g8iWmK_vgOX-iKOckNfKPafRF1mHHAg553ZHX8dTc_Dnfu31rDguXZt9mtN5Y9QZsBKd99u0A5vaDsaJjLGcfgSoQB1pwI3T8CVj4V5ppiTJLo8fU-2b7Q6kQ-vARV6lWZbmPhTAXwU7ZKDloRCHUopv1U2A")])
                                    .json(&map)
                                    .send()
                                    .await
                                    .expect("Failed to notify");
                                println!("Notify {:?}", map.get("measurement_id"));
                            }
                            Some(NotificationType::Stabilized) => {
                                let sleep_secs = {
                                    let mut rng = rand::thread_rng();
                                    rng.gen_range(0..5)
                                };
                                time::sleep(Duration::from_millis(sleep_secs*1000)).await;
                                map.insert("notification_type", "Stabilized");
                                self.client
                                    .post("http://localhost:3000/api/notify")
                                    .query(&[("token", "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJleHAiOjE3MDQxMjk4MTgsInN1YiI6ImxhbmRhdSJ9.Gvdz0IuwRPd0BloUGS9vWbRY97ZAB-HC43Fora-CFV-sejq8aNr-WNDd0mOuUP1XvgDO7gaiGv9UkOtlRBEL_TlWizDmS2buTrKWCu2C6x8U1_NR5dfjF0sKdADA4DSLxxTwiuImXNHbtkhbZjbzpMn5CYlkuydbn2Rlg4lNAV91k2zWaM4op1IQO2g8iWmK_vgOX-iKOckNfKPafRF1mHHAg553ZHX8dTc_Dnfu31rDguXZt9mtN5Y9QZsBKd99u0A5vaDsaJjLGcfgSoQB1pwI3T8CVj4V5ppiTJLo8fU-2b7Q6kQ-vARV6lWZbmPhTAXwU7ZKDloRCHUopv1U2A")])
                                    .json(&map)
                                    .send()
                                    .await
                                    .expect("Failed to notify");
                                println!("Notify {:?}", map.get("measurement_id"));
                            }
                            _ => {}
                        }

                        tokio::spawn(async move {});
                    }
                    self.consumer.commit_message(&b, CommitMode::Async).unwrap();
                }
            };
        }
    }
}
