use std::time::Duration;
use tokio::time as tktime;
use uuid::Uuid;

mod events;
mod simulator;
mod time;

use events::{KafkaTopicProducer, RecordData};
use simulator::{Experiment, ExperimentStage, TempRange};

#[tokio::main]
async fn main() {
    /* Configuration */
    let brokers: &str = "localhost:43489";
    let topic_name: &str = "experiment";
    let topic_producer = KafkaTopicProducer::new(brokers, topic_name);

    let experiment_id = Uuid::new_v4();
    let experiment_id = format!("{}", experiment_id);
    let researcher = "d.landau@uu.nl";
    let sensors: Vec<String> = vec![
        String::from("66cc5dc0-d75a-40ee-88d5-0308017191af"),
        String::from("ac5e0ea2-a04d-4eb3-a6e3-206d47ffe9e1"),
    ];
    let temp_range = TempRange::new(25.5, 26.5).unwrap();
    let sample_rate = 100;
    /* End Configuration */

    let mut experiment = Experiment::new(6.0, temp_range);

    // Experiemnt Configured Event
    let record = RecordData {
        payload: events::experiment_configured_event(
            &experiment_id,
            &researcher,
            &sensors,
            25.5,
            26.5,
        ),
        key: Some(&experiment_id),
    };
    let delivery_result = topic_producer.send_event(record).await;
    println!("Experiment Configured Event {:?}", delivery_result);
    tktime::sleep(Duration::from_millis(2000)).await;

    // Stabilization Started
    experiment.set_stage(ExperimentStage::Stabilization);
    let record = RecordData {
        payload: events::stabilization_started_event(&experiment_id),
        key: Some(&experiment_id),
    };
    let delivery_result = topic_producer.send_event(record).await;
    println!("Stabilization Started Event {:?}", delivery_result);

    // Stabilization Temperature Samples
    let stabilization_samples = experiment.stabilization_samples(2);
    let stabilization_events =
        events::temperature_events(stabilization_samples, &experiment_id, &researcher, &sensors);
    for sensor_events in stabilization_events {
        for event in sensor_events {
            let record = RecordData {
                payload: event,
                key: Some(&experiment_id),
            };
            let delivery_result = topic_producer.send_event(record).await;
            println!("sensor measurement result {:?}", delivery_result);
        }
        println!("Temperature Measured Events");
        tktime::sleep(Duration::from_millis(sample_rate)).await;
    }

    // Experiment Started
    experiment.set_stage(ExperimentStage::CarryOut);
    let record = RecordData {
        payload: events::experiment_started_event(&experiment_id),
        key: Some(&experiment_id),
    };
    let delivery_result = topic_producer.send_event(record).await;
    println!("Experiment Started Event {:?}", delivery_result);

    let carry_out_samples = experiment.carry_out_samples(20);
    let carry_out_events =
        events::temperature_events(carry_out_samples, &experiment_id, &researcher, &sensors);
    for sensor_events in carry_out_events {
        for event in sensor_events {
            let record = RecordData {
                payload: event,
                key: Some(&experiment_id),
            };
            let delivery_result = topic_producer.send_event(record).await;
            println!("sensor measurement result {:?}", delivery_result);
        }
        println!("Temperature Measured Events\n\n");
        tktime::sleep(Duration::from_millis(sample_rate)).await;
    }

    // Experiment Terminated
    experiment.set_stage(ExperimentStage::Terminated);
    let record = RecordData {
        payload: events::experiment_terminated_event(&experiment_id),
        key: Some(&experiment_id),
    };
    let delivery_result = topic_producer.send_event(record).await;
    println!("Experiment Terminated Event {:?}", delivery_result);
}
