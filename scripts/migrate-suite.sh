#! /usr/bin/env bash

OLD_DATA_DIR=$1
OLD_MANIFEST="$OLD_DATA_DIR/manifest.json"
NEW_DATA_DIR=$2

mkdir $NEW_DATA_DIR

jq -r '.entries[] | "\(.input_id)\t\(.image_id)\t\(.request_id)"' "$OLD_MANIFEST" | while IFS=$'\t' read -r input_id image_id request_id; do
    echo "Processing: $description with input_id: $input_id"

    image_path="${OLD_DATA_DIR}/images/${image_id}"
    input_path="${OLD_DATA_DIR}/inputs/${input_id}"

    RUST_LOG=debug ./target/debug/bento-bench prepare-local \
    --image "${image_path}.elf" \
    --input "${input_path}.input" \
    --data-dir "$NEW_DATA_DIR" \
    --description "Order Generator (Request: ${request_id})"
done
