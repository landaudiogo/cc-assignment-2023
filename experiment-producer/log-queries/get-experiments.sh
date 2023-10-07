#!/bin/bash


USAGE='
Usage: get-experiments.sh <log-file> <time-filter>

    log-file: Log file containing json structured data
    time-filter: Timestamp in the format displayed in the standard output by the experiment-producer

E.g.: experiment-producer/log-queries/get-experiments.sh "producer.json.log.2023-10-07" "2023-10-07T15:53:18.160161+02"
'

if (( "$#" != 2 )); then
    echo "$USAGE"
    exit 1
fi
jq --arg timefilter "$2" \
    'select(
        (.timestamp | sub("\\..+"; "Z") | fromdateiso8601)
        >= ($timefilter | sub("\\..+"; "Z") | fromdateiso8601)
    ) | select(.span.experiment_id != null) | .span.experiment_id' \
    "$1" \
    | uniq

