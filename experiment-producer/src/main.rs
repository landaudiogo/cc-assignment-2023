use uuid::Uuid;

mod events;
mod simulator;
mod time;

use events::KafkaTopicProducer;
use simulator::{Experiment, ExperimentConfiguration, TempRange};

#[tokio::main]
async fn main() {
    let brokers: &str = "localhost:43489";
    let topic_name: &str = "experiment";
    let topic_producer = KafkaTopicProducer::new(brokers, topic_name);

    let experiment_config = ExperimentConfiguration {
        experiment_id: format!("{}", Uuid::new_v4()),
        researcher: "d.landau@uu.nl".into(),
        sensors: vec![
            String::from("66cc5dc0-d75a-40ee-88d5-0308017191af"),
            String::from("ac5e0ea2-a04d-4eb3-a6e3-206d47ffe9e1"),
        ],
        sample_rate: 100,
    };

    let mut experiment = Experiment::new(
        6.0,
        TempRange::new(25.5, 26.5).unwrap(),
        experiment_config,
        topic_producer,
    );
    experiment.run().await;
}
