#!/bin/bash

packages=(
    notifications-service 
    exepriment-producer 
    http-load-generator 
    notifications-service 
    notifier 
    test-to-api
)

for package in ${packages[@]}; do
    echo "=== Pushing $package ==="
    docker push "dclandau/demo-${package}"
done

