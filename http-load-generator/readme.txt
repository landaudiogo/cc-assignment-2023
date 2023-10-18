Usage: http-load-generator [OPTIONS] --brokers <broker-list> --topic <topic> --group-id <group-id> --hosts-file <hosts-file>

E.g.: cargo run -p http-load-generator -- \
    --brokers 13.49.128.80:19093,13.49.128.80:29093,13.49.128.80:39093 \
    --topic experiment-document \
    --group-id rand-20231018 \
    --consumer-wait-before-send 0 \
    --hosts-file ./http-load-generator/hosts.json \
    --requestor-lag 0 \
    --requestor-retries 0 \
    --stable-rate-duration 10

Options:
      --secret-key <secret-key>
          <key> is a 32 character string that must match the key being passed to the notifications-service [default: QJUHsPhnA0eiqHuJqsPgzhDozYO4f1zh]
  -b, --brokers <broker-list>
          <broker-list> is a comma-seperated list of brokers. E.g.  For a single local broker `localhost:9092`. For multiple brokers `localhost:9092,localhost:9093`
      --topic <topic>
          [default: experiment]
      --group-id <group-id>

      --consumer-wait-before-send <consumer-wait-before-send>
          Time the consumer should wait before forwarding the experiment to the receiver [default: 60]
      --hosts-file <hosts-file>
          The file containing the list of hosts to be queried
      --requestor-lag <requestor-lag>
          Time the requestor lags behind the generator. [default: 5]
      --requestor-retries <requestor-retries>
          The number of retries in case a request fails due to a server error. [default: 2]
      --requestor-max-in-flight <requestor-max-in-flight>
          The maximum number of connections to a host. [default: 50]
      --min-batch-size <min-batch-size>
          The minimum number of queries that has to be performed per second to each host. [default: 100]
      --max-batch-size <max-batch-size>
          The maximum number of queries that can be performed per second to each host. [default: 200]
      --stable-rate-duration <stable-rate-duration>
          The number of seconds during which the rate at which the queries are performed to each host remains stable. [default: 60]
  -h, --help
          Print help
  -V, --version
          Print version

