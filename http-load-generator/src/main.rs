use clap::{command, Arg, ArgAction};
use futures::future;
use std::process;
use tokio::sync::mpsc;

mod consumer;
mod generator;
mod metric;
mod receiver;
mod requests;

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
        .get_matches();
    // requests::make_request().await?;

    let mut handles = vec![];
    let (tx, rx) = mpsc::channel(1000);
    handles.push(tokio::spawn(async move {
        receiver::start(rx).await;
    }));
    handles.push(tokio::spawn(async move {
        consumer::start(
            &matches
                .remove_one::<String>("broker-list")
                .expect("Required"),
            &matches.remove_one::<String>("group-id").expect("Required"),
            &vec![matches
                .remove_one::<String>("topic")
                .expect("Required")
                .as_str()],
            tx,
        )
        .await;
    }));
    future::join_all(handles).await;
    Ok(())
}
