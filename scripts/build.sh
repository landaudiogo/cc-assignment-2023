#!/bin/bash

packages=(
    notifications-service 
    experiment-producer 
    http-load-generator 
    notifier 
    test-to-api
)

for package in ${packages[@]}; do
    echo "=== Building $package ==="
    docker build --build-arg="PACKAGE=${package}" -t "dclandau/demo-${package}" .
done

