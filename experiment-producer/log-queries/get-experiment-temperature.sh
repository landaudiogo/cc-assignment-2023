#!/bin/bash


USAGE='
Usage: get-experiment-temperature.sh <log-file> <experiment-id>

    log-file: Log file containing json structured data
    experiment-id: The experiment whose measurements are to be printed

E.g.: get-experiment-temperature.sh "producer.json.log.2023-10-07" "2b9348c8-9051-4b27-a929-b5a93480fb82"
'

if (( "$#" != 2 )); then
    exit 1
fi

jq \
    --arg experiment "$2" \
    'select(.span.experiment_id == $experiment)' "$1" \
    | jq 'select(.fields.avg_temperature != null)' \
    | jq '{"timestamp": .timestamp, "avg_temperature": .fields.avg_temperature}' \
    | jq -s 

