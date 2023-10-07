use ::time::{format_description, UtcOffset};
use clap::{command, value_parser, Arg, ArgAction, ArgMatches};
use futures::future;
use tokio::time::{self as tktime, Duration};
use tracing::{span, Instrument, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{filter::LevelFilter, fmt::time::OffsetTime, prelude::*};

mod config;
mod events;
mod simulator;
mod time;

use config::ConfigFile;
use events::KafkaTopicProducer;
use simulator::{Experiment, ExperimentConfiguration, TempRange};

async fn run_single_experiment(mut matches: ArgMatches) {
    let topic_producer = KafkaTopicProducer::new(
        &matches
            .remove_one::<String>("broker-list")
            .expect("required"),
        &matches.remove_one::<String>("topic").expect("required"),
    );

    let experiment_config = ExperimentConfiguration::new(
        "d.landau@uu.nl".into(),
        matches
            .remove_one::<u8>("num-sensors")
            .expect("required")
            .into(),
        matches.remove_one::<u64>("sample-rate").expect("required"),
        TempRange::new(
            matches
                .remove_one::<f32>("lower-threshold")
                .expect("required"),
            matches
                .remove_one::<f32>("upper-threshold")
                .expect("required"),
        )
        .expect("upper-threshold should be higher than lower_threshold"),
        matches
            .remove_one::<u16>("stabilization-samples")
            .expect("required"),
        matches
            .remove_one::<u16>("carry-out-samples")
            .expect("required"),
        matches
            .remove_one::<String>("secret-key")
            .expect("required"),
    );

    let start_temperature = matches
        .remove_one::<f32>("start-temperature")
        .expect("required");

    let span = span!(
        Level::INFO,
        "experiment",
        experiment_id = experiment_config.experiment_id
    );
    let mut experiment = Experiment::new(start_temperature, experiment_config, topic_producer);
    experiment.run().instrument(span).await;
}

async fn run_multiple_experiments(mut matches: ArgMatches, config_file: &str) {
    let topic_producer = KafkaTopicProducer::new(
        &matches
            .remove_one::<String>("broker-list")
            .expect("required"),
        &matches.remove_one::<String>("topic").expect("required"),
    );

    let config = ConfigFile::from_file(config_file);
    let mut handles = vec![];

    for mut entry in config.0 {
        let start_temperature = entry.start_temperature;
        let start_offset = entry.start_time;
        entry.set_secret_key(&matches.get_one::<String>("secret-key").expect("required"));
        let experiment_config = ExperimentConfiguration::from(entry);
        let topic_producer = topic_producer.clone();

        let span = span!(
            Level::INFO,
            "experiment",
            experiment_id = experiment_config.experiment_id
        );
        handles.push(tokio::spawn(
            async move {
                tktime::sleep(Duration::from_millis(start_offset * 1000)).await;

                let mut experiment =
                    Experiment::new(start_temperature, experiment_config, topic_producer);
                experiment.run().await;
            }
            .instrument(span),
        ));
    }
    future::join_all(handles).await;
}

fn configure_tracing() -> WorkerGuard {
    let mut layers = vec![];

    let offset = UtcOffset::from_hms(2, 0, 0).expect("Should get CET offset");
    let time_format = format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6][offset_hour sign:mandatory]",
    )
    .expect("format string should be valid");
    let timer = OffsetTime::new(offset, time_format);

    let file_appender = tracing_appender::rolling::daily("./", "producer.json.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    layers.push(
        tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_writer(non_blocking)
            .with_timer(timer.clone())
            .json()
            .with_filter(LevelFilter::DEBUG)
            .boxed(),
    );

    layers.push(
        tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_timer(timer)
            .with_filter(LevelFilter::INFO)
            .boxed(),
    );

    tracing_subscriber::registry().with(layers).init();
    _guard
}

fn configure_cli() -> ArgMatches {
    command!() // requires `cargo` feature
        .next_line_help(true)
        .arg(Arg::new("secret-key")
            .required(false)
            .long("secret-key")
            .action(ArgAction::Set)
            .default_value("QJUHsPhnA0eiqHuJqsPgzhDozYO4f1zh")
            .help("<key> is a 32 character string that must match the key being passed to the notifications-service")
        )
        .arg(Arg::new("config-file")
            .required(false)
            .action(ArgAction::Set)
            .long("config-file")
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
        .get_matches()
}

#[tokio::main]
async fn main() {
    let _guard = configure_tracing();
    let mut matches = configure_cli();

    if let Some(config_file) = matches.remove_one::<String>("config-file") {
        run_multiple_experiments(matches, &config_file).await;
    } else {
        run_single_experiment(matches).await;
    }
}
