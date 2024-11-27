#!/bin/bash
# run-taint.sh

if [ $# -ne 4 ]; then
    echo "Usage: $0 <lir-file> <json-file> <pointer-solution> <context-sensitivity>"
    echo "Context sensitivity options: ci, functional, or callstring-k (where k is a number)"
    exit 1
fi

lir_file="$1"
json_file="$2"
pts_file="$3"
context="$4"

# Run the taint analysis
./target/release/taint "$lir_file" "$json_file" "$pts_file" "$context"