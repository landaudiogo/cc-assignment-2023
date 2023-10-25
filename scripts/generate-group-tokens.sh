#!/bin/bash

USAGE="
Usage: generate-group-tokens.sh <num-groups>

    num-groups: integer number of groups
"

if (( $# != 1 )); then
    cat <<< "$USAGE"
    exit 1
fi 


if ! [[ "$1" =~ ^[0-9]+$ ]]; then
    cat <<< "$USAGE"
    exit 1
fi

num_groups="$1"

for i in $(seq 1 $num_groups); do
    echo "=== group$i ==="
    cargo run -p notifications-service --bin generate-jwt-token -- --client-id "group${i}" > "../kafka-dev/creds/groups/group${i}/token"
done
