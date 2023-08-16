use apache_avro::Reader;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

mod events;

mod simulator; 
use simulator::TempRange;


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

    producer
        .send(
            FutureRecord::to(topic_name)
                .payload(&events::experiment_configured_event(
                    &experiment_id,
                    &researcher,
                    &sensors,
                    25.5,
                    26.5,
                ))
                .key(&experiment_id),
            Duration::from_secs(0),
        )
        .await.unwrap();
    println!("Experiment Configured Event");

    producer
        .send(
            FutureRecord::to(topic_name)
                .payload(&events::stabilization_started_event(&experiment_id))
                .key(&experiment_id),
            Duration::from_secs(0),
        )
        .await.unwrap();
    println!("Stabilization Started Event");

    let stabilization_samples = simulator::stabilization_samples(6.0, temp_range, 2);
    let stabilization_events =
        events::stabilization_events(stabilization_samples, &experiment_id, &sensors, temp_range);
    for sensor_events in stabilization_events {
        for event in sensor_events {
            producer
                .send(
                    FutureRecord::to(topic_name)
                        .payload(&event)
                        .key(&experiment_id),
                    Duration::from_secs(0),
                )
                .await.unwrap();
            let reader = Reader::new(&event.0[..]).unwrap();
            for value in reader {
                println!("{:?}", value);
            }
        }
        println!("Temperature Measured Events");
        time::sleep(Duration::from_millis(2000)).await;
    }

    let delivery_status = producer
        .send(
            FutureRecord::to(topic_name)
                .payload(&events::experiment_started_event(&experiment_id))
                .key(&experiment_id),
            Duration::from_secs(0),
        )
        .await;
    println!("delivery status {:?}", delivery_status);

    let delivery_status = producer
        .send(
            FutureRecord::to(topic_name)
                .payload(&events::experiment_terminated_event(&experiment_id))
                .key(&experiment_id),
            Duration::from_secs(0),
        )
        .await;
    println!("delivery status {:?}", delivery_status);
}
