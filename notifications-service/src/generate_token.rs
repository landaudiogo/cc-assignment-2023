use clap::{command, Arg, ArgAction};

mod jwt;
use crate::jwt::{encode, Claims};

fn main() {
    let mut matches = command!() // requires `cargo` feature
        .next_line_help(true)
        .arg(
            Arg::new("client-id")
                .required(true)
                .long("client-id")
                .action(ArgAction::Set)
                .default_value("group0"),
        )
        .get_matches();

    let claims = Claims::new(matches.remove_one::<String>("client-id").expect("required"));
    let token = encode(&claims);
    println!("{:?}", token);
}
