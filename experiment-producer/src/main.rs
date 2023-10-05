use uuid::Uuid;
use clap::{command, ArgAction, Arg, value_parser};
use tokio::time::{self as tktime, Duration};
use futures::future;

mod events;
mod simulator;
mod time;

use events::KafkaTopicProducer;
use simulator::{Experiment, ExperimentConfiguration, TempRange};


#[tokio::main]
async fn main() {
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
            .required(false)
            .long("topic")
            .default_value("experiment")
            .action(ArgAction::Set)
        )
        .arg(Arg::new("num-sensors")
            .required(false)
            .long("num-sensors")
            .default_value("2")
            .action(ArgAction::Set)
            .value_parser(value_parser!(u8))
        )
        .arg(Arg::new("sample-rate")
            .required(false)
            .long("sample-rate")
            .default_value("100")
            .action(ArgAction::Set)
            .value_parser(value_parser!(u64))
        )
        .arg(Arg::new("stabilization-samples")
            .required(false)
            .long("stabilization-samples")
            .default_value("2")
            .action(ArgAction::Set)
            .value_parser(value_parser!(u16))
        )
        .arg(Arg::new("carry-out-samples")
            .required(false)
            .long("carry-out-samples")
            .default_value("20")
            .action(ArgAction::Set)
            .value_parser(value_parser!(u16))
        )
        .arg(Arg::new("start-temperature")
            .required(false)
            .long("start-temperature")
            .default_value("16")
            .action(ArgAction::Set)
            .value_parser(value_parser!(f32))
        )
        .arg(Arg::new("lower-threshold")
            .required(false)
            .long("lower-threshold")
            .default_value("25.5")
            .action(ArgAction::Set)
            .value_parser(value_parser!(f32))
        )
        .arg(Arg::new("upper-threshold")
            .required(false)
            .long("upper-threshold")
            .default_value("26.5")
            .action(ArgAction::Set)
            .value_parser(value_parser!(f32))
        )
        .get_matches();

    let topic_producer = KafkaTopicProducer::new(
        &matches.remove_one::<String>("broker-list").expect("required"), 
        &matches.remove_one::<String>("topic").expect("required"),
    );

    let num_sensors = matches.remove_one::<u8>("num-sensors").expect("required");

    let experiment_config = ExperimentConfiguration {
        experiment_id: format!("{}", Uuid::new_v4()),
        researcher: "d.landau@uu.nl".into(),
        sensors: (0..num_sensors).map(|_| format!("{}", Uuid::new_v4())).collect(),
        sample_rate: matches.remove_one::<u64>("sample-rate").expect("required"),
        secret_key: matches.remove_one::<String>("secret-key").expect("required").clone(),
        temp_range: TempRange::new(
            matches.remove_one::<f32>("lower-threshold").expect("required"), 
            matches.remove_one::<f32>("upper-threshold").expect("required"), 
        ).unwrap(),
        stabilization_samples: matches.remove_one::<u16>("stabilization-samples").expect("required"),
        carry_out_samples: matches.remove_one::<u16>("carry-out-samples").expect("required"),
    };

    let start_temperature = matches.remove_one::<f32>("start-temperature").expect("required");
    let start_time = time::current_epoch();
    let start_offsets = vec![1, 5];
    let mut handles = vec![];

    for task_sleep in start_offsets.into_iter() {
        let topic_producer = topic_producer.clone();
        let experiment_config = experiment_config.clone();
        let start_temperature = start_temperature.clone();

        handles.push(tokio::spawn(async move {
            tktime::sleep(Duration::from_millis(task_sleep * 1000)).await;
            let current_time = time::current_epoch();
            println!("{} {}", current_time - start_time, task_sleep);

            let mut experiment = Experiment::new(
                start_temperature, 
                experiment_config,
                topic_producer,
            );
            experiment.run().await;

        }));
    }
    future::join_all(handles).await;
}
