use clap::{command, Arg, ArgAction};
use crate::{consume::{ConsumeConfiguration, Consume}};

mod consume;

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

    let consume_config = ConsumeConfiguration::from(&mut matches);
    let consume = Consume::new(consume_config);
    consume.start().await;
}
