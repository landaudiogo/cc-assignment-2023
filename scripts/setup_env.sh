#!/bin/bash

copy_file_to_dst() {
    dir="$(dirname $2)"
    file="$(basename $2)"
    if ! [[ -d "$dir" ]]; then
        mkdir -p $dir
    fi 
    cp "$1" $2
}

USAGE='
Usage: setup_config.sh <environment>

    environment: One of the environments available on the repository'"'"'s root
        directory `<repository-root>/config` path
'

if [[ -z "$1" ]]; then
    echo "$USAGE"
    exit 1
fi 

script_dir="$(cd -- $(dirname ${BASH_SOURCE[0]}) && pwd)"
repo_dir="$(dirname "$script_dir")"

environment="${repo_dir}/config/$1"
if ! [[ -d "$environment" ]]; then
    echo "$USAGE"
    exit 1
fi

files=$(find "$environment" -type f )
for src in $files; do
    relative_file="${src#"$environment/"}"
    dst="${repo_dir}/${relative_file}"
    copy_file_to_dst "$src" "$dst"
done
