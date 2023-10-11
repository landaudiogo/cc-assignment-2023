use apache_avro::{from_value, Reader};
use rdkafka::{
    client::ClientContext,
    config::ClientConfig,
    consumer::stream_consumer::StreamConsumer,
    consumer::{CommitMode, Consumer, ConsumerContext, Rebalance},
    message::Message,
};
use serde::Deserialize;
use tokio::{
    sync::mpsc::Sender,
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
struct Measurement {
    timestamp: f64,
    temperature: f32,
}

#[derive(Debug, Deserialize, Clone)]
struct TempRange {
    upper_threshold: f32,
    lower_threshold: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExperimentDocument {
    pub experiment: String,
    measurements: Vec<Measurement>,
    temperature_range: TempRange,
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
                    let experiment_document: ExperimentDocument =
                        from_value(&value.unwrap()).expect("Received invalid event");
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
