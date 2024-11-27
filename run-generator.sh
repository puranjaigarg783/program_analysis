#!/bin/bash
# run-generator.sh
if [ $# -ne 2 ]; then
    echo "Usage: $0 <lir-file> <json-file>"
    exit 1
fi

lir_file="$1"
json_file="$2"

./target/release/constraint_gen "$lir_file" "$json_file"