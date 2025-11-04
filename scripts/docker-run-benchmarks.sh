#!/usr/bin/env bash

set -eu

BENCHER_CMD=/app/bencher
DATA_DIR=/data

for elf_file in "$DATA_DIR"/*.elf; do
    id="$(basename "$elf_file" .elf)"
    in_file="${elf_file%.elf}.input"

    echo "Running benchmark: $id"

    "$BENCHER_CMD" bench -f "$elf_file" -i "$in_file"
done