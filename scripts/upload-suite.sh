#!/usr/bin/env bash

set -ux

DATA_DIR=./data
MANIFEST=./manifest.json
SUITE_NAME=$1
TAR_FILE="$SUITE_NAME".tar.zst

R2_ENDPOINT="https://31470fca903ed77d898151ffc4a2a807.r2.cloudflarestorage.com/"
R2_BUCKET="boundless-benchmarks"
R2_PATH="suites"

tar -caf "$TAR_FILE" "$DATA_DIR/*"

AWS_ACCESS_KEY_ID="$R2_ACCESS_KEY" \
AWS_SECRET_ACCESS_KEY="$R2_SECRET_KEY" \
aws s3api put-object \
  --body "$TAR_FILE" \
  --endpoint-url "$R2_ENDPOINT" \
  --key ${R2_PATH}/${TAR_FILE} \
  --bucket "$R2_BUCKET" \
  --no-cli-pager