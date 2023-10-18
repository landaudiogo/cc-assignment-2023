Usage: test-to-api [OPTIONS] --brokers <broker-list> --topic <topic> --group-id <group-id>

E.g.: cargo run -p test-to-api -- \
    --brokers 13.49.128.80:19093,13.49.128.80:29093,13.49.128.80:39093 \
    --topic experiment-document \
    --group-id rand-202310181830

Options:
      --secret-key <secret-key>
          <key> is a 32 character string that must match the key being passed to the notifications-service [default: QJUHsPhnA0eiqHuJqsPgzhDozYO4f1zh]
  -b, --brokers <broker-list>
          <broker-list> is a comma-seperated list of brokers. E.g.  For a single local broker `localhost:9092`. For multiple brokers `localhost:9092,localhost:9093`
      --topic <topic>
          [default: experiment]
      --group-id <group-id>

  -h, --help
          Print help
  -V, --version
          Print version
