#!/bin/bash
# run-slice.sh
if [ $# -ne 4 ]; then
    echo "Usage: $0 <lir-file> <json-file> <function#bb#{index|term}> <pointer-solution>"
    exit 1
fi

lir_file="$1"
json_file="$2"
criterion="$3"   # This should be in format: function#bb#index or function#bb#term
pts_file="$4"

./target/release/slice "$lir_file" "$json_file" "$criterion" "$pts_file"