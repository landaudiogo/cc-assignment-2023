use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;
use apache_avro::types::{Record, Value};
use apache_avro::{Schema, Writer};
use rdkafka::message::ToBytes;
use uuid::Uuid;
use rand::Rng;

use crate::simulator::{
    IntoIter, TempRange
};

/// `Vec<u8>` wrapper
///
/// FutureRecord::payload requires a type that implements the trait `ToBytes` as an argument. This is our
/// custom type to implement the trait.
pub struct EventWrapper(pub Vec<u8>);

impl<'a> ToBytes for EventWrapper {
    fn to_bytes(&self) -> &[u8] {
        &self.0
    }
}

pub fn experiment_configured_event(
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

pub fn stabilization_started_event(experiment_id: &str) -> EventWrapper {
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

pub fn experiment_started_event(experiment_id: &str) -> EventWrapper {
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

pub fn experiment_terminated_event(experiment_id: &str) -> EventWrapper {
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

pub fn temperature_measured_event(
    experiment: &str,
    measurement_id: &str,
    sensor: &str,
    temperature: f32,
    timestamp: f64,
    measurement_hash: &str,
) -> EventWrapper {
    let raw_schema =
        fs::read_to_string("experiment-producer/schemas/sensor_temperature_measured.avro").unwrap();
    let schema = Schema::parse_str(&raw_schema).unwrap();
    let mut writer = Writer::new(&schema, Vec::new());

    let mut record = Record::new(writer.schema()).unwrap();
    record.put("experiment", experiment);
    record.put("sensor", sensor);
    record.put("measurement_id", measurement_id);
    record.put("temperature", temperature);
    record.put("measurement_hash", measurement_hash);
    record.put("timestamp", Value::Double(timestamp));

    writer.append(record).unwrap();
    EventWrapper(writer.into_inner().unwrap())
}

pub fn stabilization_events<'a>(
    sample_iter: IntoIter,
    experiment_id: &'a str,
    sensors: &'a Vec<String>,
    temp_range: TempRange,
) -> Box<dyn Iterator<Item = Vec<EventWrapper>> + 'a> {
    let mut prev_sample = None;

    Box::new(sample_iter.map(move |sample| {
        // let prev = prev_sample.unwrap();
        let measurement_id = &format!("{}", Uuid::new_v4());
        let measurement_hash = "abcd.efgh";
        let current_time = SystemTime::now();
        let current_time = current_time
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let current_time: f64 =
            current_time.as_secs() as f64 + current_time.subsec_nanos() as f64 / 1_000_000_000_f64;

        let mut total_temperature = 0.0;
        prev_sample = Some(sample);
        let mut sensor_events = sensors[..sensors.len() - 1]
            .iter()
            .map(|sensor_id| {
                let relative_diff = rand::thread_rng().gen_range(-100.0..100.0);
                let sensor_temperature = sample.cur() + relative_diff * 1.0 / 100.0;
                total_temperature += sensor_temperature;
                temperature_measured_event(
                    experiment_id,
                    measurement_id,
                    sensor_id,
                    sensor_temperature,
                    current_time,
                    measurement_hash,
                )
            })
            .collect::<Vec<EventWrapper>>();
        let last_sensor_id = &sensors[sensors.len() - 1];
        let last_sensor_temperature = (sensors.len() as f32) * sample.cur() - total_temperature;
        sensor_events.push(temperature_measured_event(
            experiment_id,
            measurement_id,
            &last_sensor_id,
            last_sensor_temperature,
            current_time,
            measurement_hash,
        ));
        sensor_events
    }))
}

