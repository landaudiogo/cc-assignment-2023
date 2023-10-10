use apache_avro::{
    Reader, from_value
};
use rdkafka::{
    client::ClientContext,
    config::ClientConfig,
    consumer::stream_consumer::StreamConsumer,
    consumer::{CommitMode, Consumer, ConsumerContext, Rebalance},
    message::Message,
};
use clap::{command, Arg, ArgAction};
use serde::Deserialize;
use reqwest::header::{CONTENT_TYPE, ACCEPT};

struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {
    fn post_rebalance(&self, rebalance: &Rebalance) {
        println!("Post rebalance {:?}", rebalance);
    }
}

#[derive(Debug, Deserialize)]
struct Measurement {
    timestamp: f64, 
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct TempRange {
    upper_threshold: f32, 
    lower_threshold: f32,
}


#[derive(Debug, Deserialize)]
struct ExperimentDocument {
    experiment: String, 
    measurements: Vec<Measurement>,
    temperature_range: TempRange
}

async fn consume_and_print(brokers: &str, group_id: &str, topics: &[&str]) {
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

    loop {
        match consumer.recv().await {
            Err(e) => println!("Kafka error: {}", e),
            Ok(b) => {
                let m = b.detach();
                let reader = Reader::new(m.payload().unwrap()).unwrap();
                for value in reader {
                    println!("{:?}", from_value::<ExperimentDocument>(&value.unwrap()));
                }
                consumer.commit_message(&b, CommitMode::Async).unwrap();
            }
        };
    }
}

async fn make_request() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client
        .post("http://127.0.0.1:3000/api/notify")
        .header(CONTENT_TYPE, "application/json; charset=utf-8")
        .header(ACCEPT, "text/plain; chaset=utf-8")
        .body(r#"{
            "notification_type": "OutOfRange",
            "researcher": "d.landau@uu.nl",
            "measurement_id": "1234",
            "experiment_id": "5678",
            "cipher_data": "D5qnEHeIrTYmLwYX.hSZNb3xxQ9MtGhRP7E52yv2seWo4tUxYe28ATJVHUi0J++SFyfq5LQc0sTmiS4ILiM0/YsPHgp5fQKuRuuHLSyLA1WR9YIRS6nYrokZ68u4OLC4j26JW/QpiGmAydGKPIvV2ImD8t1NOUrejbnp/cmbMDUKO1hbXGPfD7oTvvk6JQVBAxSPVB96jDv7C4sGTmuEDZPoIpojcTBFP2xA"
        }"#)
        .send()
        .await
        .unwrap();
    println!("{:?}", res.text_with_charset("utf-8").await);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    // make_request().await?;
    consume_and_print(
        &matches.remove_one::<String>("broker-list").expect("Required"),
        &matches.remove_one::<String>("group-id").expect("Required"),
        &vec![matches.remove_one::<String>("topic").expect("Required").as_str()],
    ).await;
    Ok(())
}

