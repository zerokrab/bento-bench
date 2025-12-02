#!/usr/bin/env bash

set -eu

MANIFEST_PATH=/manifest.json
DATA_DIR=/data

/app/bento-bench run \
    --manifest /manifest.json
    --data-dir /data
