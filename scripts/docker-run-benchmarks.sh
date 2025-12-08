#!/usr/bin/env bash

set -eu

DATA_DIR=/data

/app/bento-bench run \
    --data-dir "$DATA_DIR"
