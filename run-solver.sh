#!/bin/bash
# run-solver.sh
if [ $# -ne 1 ]; then
    echo "Usage: $0 <constraints-file>"
    exit 1
fi

constraints_file="$1"

./target/release/constraint_solve "$constraints_file"