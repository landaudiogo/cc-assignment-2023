use clap::{command, value_parser, Arg, ArgAction};
use futures::future;
use std::process;
use tokio::sync::mpsc;

mod consume;
mod experiment;
mod generator;
mod metric;
mod receiver;
mod requests;

use crate::consume::{Consume, ConsumeConfiguration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

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
        .arg(Arg::new("consumer-wait-before-send")
            .required(false)
            .long("consumer-wait-before-send")
            .action(ArgAction::Set)
            .default_value("60")
            .value_parser(value_parser!(u8))
            .help("Time the consumer should wait before forwarding the experiment to the receiver")
        )
        .get_matches();

    let consume_config = ConsumeConfiguration::from(&mut matches);
    let consume = Consume::new(consume_config);

    let mut handles = vec![];
    let (tx, rx) = mpsc::channel(1000);
    handles.push(tokio::spawn(async move {
        receiver::start(rx).await;
    }));
    handles.push(tokio::spawn(async move {
        consume.start(tx).await;
    }));
    future::join_all(handles).await;
    Ok(())
}
