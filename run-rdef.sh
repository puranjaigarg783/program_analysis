# Build the project
# cargo build --release

#!/bin/bash
# run-rdef.sh
if [ $# -ne 3 ]; then
    echo "Usage: $0 <lir-file> <json-file> <function-name>"
    exit 1
fi

lir_file="$1"
json_file="$2"
func_name="$3"

# Run the reaching definitions analysis
./target/release/rdef "$lir_file" "$json_file" "$func_name"
