#!/usr/bin/env bash

set -eu

MANIFEST_PATH=/manifest.json
DATA_DIR=/data

/app/bento-bench run \
    --manifest "$MANIFEST_PATH" \
    --data-dir "$DATA_DIR"
