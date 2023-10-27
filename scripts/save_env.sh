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
Usage: save_env.sh <environment>

    environment: Save the current configuration files into an environment in the repository'"'"'s root `config` directory
'

if [[ -z "$1" ]]; then
    echo "$USAGE"
    exit 1
fi 

script_dir="$(cd -- $(dirname ${BASH_SOURCE[0]}) && pwd)"
repo_dir="$(dirname "$script_dir")"


template="${repo_dir}/config/template"
environment="${repo_dir}/config/$1"
files=$(find "$template" -type f )
for file in $files; do
    relative_file="${file#"$template/"}"
    src="${repo_dir}/${relative_file}"
    dst="${repo_dir}/config/$1/${relative_file}"
    copy_file_to_dst "$src" "$dst"
done
