use apache_avro::types::{Record, Value};
use apache_avro::{Reader, Schema, Writer};
use rand::Rng;
use rdkafka::config::ClientConfig;
use rdkafka::message::ToBytes;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, time::Duration};
use tokio::time;
use uuid::Uuid;

/// `Vec<u8>` wrapper
///
/// FutureRecord::payload requires a type that implements the trait `ToBytes` as an argument. This is our
/// custom type to implement the trait.
struct EventWrapper(Vec<u8>);

impl<'a> ToBytes for EventWrapper {
    fn to_bytes(&self) -> &[u8] {
        &self.0
    }
}

fn experiment_configured_event(
    experiment_id: &str,
    researcher: &str,
    sensors: &Vec<String>,
    upper_threshold: f32,
    lower_threshold: f32,
) -> EventWrapper {
    let raw_schema =
        fs::read_to_string("experiment-producer/schemas/experiment_configured.avro").unwrap();
    let schema = Schema::parse_str(&raw_schema).unwrap();
    let mut writer = Writer::new(&schema, Vec::new());

    let mut record = Record::new(writer.schema()).unwrap();
    record.put("experiment", experiment_id);
    record.put("researcher", researcher);
    let sensors = Value::Array(sensors.into_iter().map(|v| (&**v).into()).collect());
    record.put("sensors", sensors);

    let schema_json: serde_json::Value = serde_json::from_str(&raw_schema).unwrap();
    let temp_schema_json = &schema_json["fields"][3]["type"];

    let temp_schema = Schema::parse_str(&temp_schema_json.to_string()).unwrap();
    let mut temp_range = Record::new(&temp_schema).unwrap();
    temp_range.put("upper_threshold", Value::Float(upper_threshold));
    temp_range.put("lower_threshold", Value::Float(lower_threshold));
    record.put("temperature_range", temp_range);
    writer.append(record).unwrap();

    EventWrapper(writer.into_inner().unwrap())
}

fn stabilization_started_event(experiment_id: &str) -> EventWrapper {
    let raw_schema =
        fs::read_to_string("experiment-producer/schemas/stabilization_started.avro").unwrap();
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

fn experiment_started_event(experiment_id: &str) -> EventWrapper {
    let raw_schema =
        fs::read_to_string("experiment-producer/schemas/experiment_started.avro").unwrap();
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

fn experiment_terminated_event(experiment_id: &str) -> EventWrapper {
    let raw_schema =
        fs::read_to_string("experiment-producer/schemas/experiment_terminated.avro").unwrap();
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

fn temperature_measured_event(
    experiment: &str,
    measurement_id: &str,
    sensor: &str,
    temperature: f32,
    measurement_hash: &str,
) -> EventWrapper {
    let raw_schema =
        fs::read_to_string("experiment-producer/schemas/sensor_temperature_measured.avro").unwrap();
    let schema = Schema::parse_str(&raw_schema).unwrap();
    let mut writer = Writer::new(&schema, Vec::new());

    let mut record = Record::new(writer.schema()).unwrap();
    record.put("experiment", experiment);
    record.put("sensor", sensor);
    record.put("measurement-id", measurement_id);
    record.put("temperature", temperature);
    record.put("measurement_hash", measurement_hash);

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

struct TempRange {
    lower_threshold: f32,
    upper_threshold: f32,
}

impl TempRange {
    fn new(lower_threshold: f32, upper_threshold: f32) -> Option<Self> {
        if lower_threshold > upper_threshold {
            return None;
        }
        Some(Self {
            upper_threshold,
            lower_threshold,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct TemperatureSample {
    cur: f32,
}

impl TemperatureSample {
    fn into_iter(self, delta: f32, len: usize, random_range: f32) -> IntoIter {
        IntoIter {
            sample: self,
            iteration: 0,
            delta,
            len,
            random_range,
        }
    }
}

struct IntoIter {
    sample: TemperatureSample,
    delta: f32,
    len: usize,
    iteration: usize,
    random_range: f32,
}

impl Iterator for IntoIter {
    type Item = TemperatureSample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iteration >= self.len {
            return None;
        }

        self.sample.cur += self.delta;
        if self.random_range != 0.0 {
            let relative_val = rand::thread_rng().gen_range(-100.0..100.0);
            let absolute_val = relative_val * self.random_range / 100.0;
            self.sample.cur += absolute_val;
        }
        self.iteration += 1;
        Some(self.sample)
    }
}

fn stabilization_samples(start: f32, temperature_range: TempRange, len: usize) -> IntoIter {
    let TempRange {
        lower_threshold,
        upper_threshold,
    } = temperature_range;
    let final_temperature = lower_threshold + (upper_threshold - lower_threshold) / 2_f32;
    let delta = (final_temperature - start) / (len as f32);
    TemperatureSample { cur: start }.into_iter(delta, len, 0.0)
}

fn stabilization_events<'a>(
    sample_iter: IntoIter,
    experiment_id: &'a str,
    sensors: &'a Vec<String>,
) -> Box<dyn Iterator<Item = Vec<EventWrapper>> + 'a> {
    let mut prev_sample = None;

    Box::new(sample_iter.map(move |sample| {
        // let prev = prev_sample.unwrap();
        let measurement_id = &format!("{}", Uuid::new_v4());
        let measurement_hash = "abcd.efgh";

        let mut total_temperature = 0.0;
        prev_sample = Some(sample);
        let mut sensor_events = sensors[..sensors.len() - 1]
            .iter()
            .map(|sensor_id| {
                let relative_diff = rand::thread_rng().gen_range(-100.0..100.0);
                let sensor_temperature = sample.cur + relative_diff * 1.0 / 100.0;
                total_temperature += sensor_temperature;
                temperature_measured_event(
                    experiment_id,
                    measurement_id,
                    sensor_id,
                    sensor_temperature,
                    measurement_hash,
                )
            })
            .collect::<Vec<EventWrapper>>();
        let last_sensor_id = &sensors[sensors.len() - 1];
        let last_sensor_temperature = sample.cur - total_temperature;
        sensor_events.push(temperature_measured_event(
            experiment_id,
            measurement_id,
            &last_sensor_id,
            last_sensor_temperature,
            measurement_hash,
        ));
        sensor_events
    }))
}

fn carry_out_samples(sample: TemperatureSample, len: usize) {}

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
        String::from("ac5e0ea2-a04d-4eb3-a6e3-206d47ffe9e1"),
    ];

    let temp_range = TempRange::new(25.5, 26.5).unwrap();
    let stabilization_samples = stabilization_samples(6.0, temp_range, 20);
    let stabilization_events =
        stabilization_events(stabilization_samples, &experiment_id, &sensors);
    for sensor_events in stabilization_events {
        for event in sensor_events {
            let delivery_status = producer
                .send(
                    FutureRecord::to(topic_name)
                        .payload(&event)
                        .key(&experiment_id),
                    Duration::from_secs(0),
                )
                .await;
            println!("Sent event {:?}", delivery_status);
            time::sleep(Duration::from_millis(100)).await;
        }
    }

    let delivery_status = producer
        .send(
            FutureRecord::to(topic_name)
                .payload(&experiment_configured_event(
                    &experiment_id,
                    &researcher,
                    &sensors,
                    25.5,
                    26.5,
                ))
                .key(&experiment_id),
            Duration::from_secs(0),
        )
        .await;
    println!("delivery status {:?}", delivery_status);

    let delivery_status = producer
        .send(
            FutureRecord::to(topic_name)
                .payload(&stabilization_started_event(&experiment_id))
                .key(&experiment_id),
            Duration::from_secs(0),
        )
        .await;
    println!("delivery status {:?}", delivery_status);

    let delivery_status = producer
        .send(
            FutureRecord::to(topic_name)
                .payload(&experiment_started_event(&experiment_id))
                .key(&experiment_id),
            Duration::from_secs(0),
        )
        .await;
    println!("delivery status {:?}", delivery_status);

    let delivery_status = producer
        .send(
            FutureRecord::to(topic_name)
                .payload(&experiment_terminated_event(&experiment_id))
                .key(&experiment_id),
            Duration::from_secs(0),
        )
        .await;
    println!("delivery status {:?}", delivery_status);
}
