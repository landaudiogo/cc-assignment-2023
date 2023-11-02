#!/bin/bash

packages=(
    notifications-service 
    experiment-producer 
    http-load-generator 
    notifier 
    test-to-api
)

for package in ${packages[@]}; do
    echo "=== Pushing $package ==="
    docker push "dclandau/demo-${package}"
done

