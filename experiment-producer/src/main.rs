use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    time::Duration,
    fs
};
use apache_avro::types::{Record, Value};
use apache_avro::{Writer, Reader, Schema};
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::message::ToBytes;
use uuid::Uuid;


struct EventWrapper(Vec<u8>);

impl<'a> ToBytes for EventWrapper {
    fn to_bytes(&self) -> &[u8] {
        &self.0 
    }
}

fn experiment_configured_event(
    experiment_id: &str, researcher: &str, sensors: &Vec<String>, 
    upper_threshold: f32, lower_threshold: f32
) -> EventWrapper {
    let raw_schema = fs::read_to_string("experiment-producer/schemas/experiment_configured.avro").unwrap();
    let schema = Schema::parse_str(&raw_schema).unwrap();
    let mut writer = Writer::new(&schema, Vec::new());

    let mut record = Record::new(writer.schema()).unwrap();
    record.put("experiment", experiment_id);
    record.put("researcher", researcher);
    let sensors = Value::Array(
        sensors
            .into_iter()
            .map(|v| (&**v).into())
            .collect()
    );
    record.put("sensors", sensors);

    let schema_json: serde_json::Value = serde_json::from_str(&raw_schema).unwrap();
    let temp_schema_json = &schema_json["fields"][3]["type"]; 

    let temp_schema = Schema::parse_str(&temp_schema_json.to_string()).unwrap();
    let mut temp_range = Record::new(&temp_schema).unwrap();
    temp_range.put("upper_threshold", Value::Float(26.0));
    temp_range.put("lower_threshold", Value::Float(25.0));
    record.put("temperature_range", temp_range);
    writer.append(record).unwrap();

    EventWrapper(writer.into_inner().unwrap())
}

fn experiment_started_event(experiment_id: &str) -> EventWrapper {
    let raw_schema = fs::read_to_string("experiment-producer/schemas/experiment_started.avro").unwrap();
    let schema = Schema::parse_str(&raw_schema).unwrap();
    let mut writer = Writer::new(&schema, Vec::new());

    let mut record = Record::new(writer.schema()).unwrap();
    record.put("experiment", experiment_id);

    let current_time = SystemTime::now();
    let current_time = current_time
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let current_time: f64 =
        current_time.as_secs() as f64 + current_time.subsec_nanos() as f64 / 1_000_000_000_f64;
    record.put("timestamp", Value::Double(current_time));
    writer.append(record).unwrap();

    EventWrapper(writer.into_inner().unwrap())
}


#[tokio::main]
async fn main() {
    let brokers: &str = "localhost:43489";
    let topic_name: &str = "experiment";
    let producer: &FutureProducer = &ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

    let experiment_id = Uuid::new_v4();
    let experiment_id = format!("{}", experiment_id);
    let researcher = "d.landau@uu.nl";
    let sensors: Vec<String> = vec![
        String::from("66cc5dc0-d75a-40ee-88d5-0308017191af"),
        String::from("ac5e0ea2-a04d-4eb3-a6e3-206d47ffe9e1")
    ];

    let delivery_status = producer
        .send(
            FutureRecord::to(topic_name)
                .payload(&experiment_configured_event(&experiment_id, &researcher, &sensors, 25.5, 26.5))
                .key(&experiment_id), 
            Duration::from_secs(0)
        ).await;
    println!("delivery status {:?}", delivery_status);

    let delivery_status = producer
        .send(
            FutureRecord::to(topic_name)
                .payload(&experiment_started_event(&experiment_id))
                .key(&experiment_id), 
            Duration::from_secs(0)
        ).await;
    println!("delivery status {:?}", delivery_status);
}
