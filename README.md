# bento-bench

## Installing

### Binaries

See [releases](https://github.com/2Boys1Proof/bento-bench/releases).

### Build From Source
```shell
git clone github.com/2boys1proof/bento-bench
cd bento-bench
cargo build --release
```

## Running Benchmarks

A collection of prepared suites are available:

| Description                               | Link                                                                              |   
|-------------------------------------------|-----------------------------------------------------------------------------------|
| Order Generator (Tiny, 1M-10M)            | https://boundless-benchmarks.mintybasil.dev/suites/og-suite-tiny_1m-10m.tar.zst   |
| Order Generator (Small, 100M-1B)          | https://boundless-benchmarks.mintybasil.dev/suites/og-suite-small_100m-1b.tar.zst |
| Order Generator (Varying sizes, 49M-3.9B) | https://boundless-benchmarks.mintybasil.dev/suites/og-suite-varied_49m-4b.tar.zst |

To fetch and untar:
```shell
curl <link> | tar -xv --zstd
```

Once you have a manifest and data directory, benchmarks can be run with
```shell
bento-bench run \
    --manifest manifest.json \
    --data-dir ./data
```

To configure the bento backend, see `bento-bench run --help`. The summary of the benchmarks can be outputted to a json file with `--json <path>`.

## Docker

```shell
docker run --mount <data-path>:/data <manifest-path>:/manifest.json ghcr.io/2boys1proof/bento-bench 
```


## Creating Benchmarks

### Fetching Market Requests

```shell
bento-bench prepare-request \
    --manifest manifest.json \
    --request-id 0x1234...ABCD \
    --description "A simple request (500M)" \
    --data-dir ./data
    --rpc-url "http://node:8545"
```

This will fetch and save the image and input to the data dir, and append them to the manifest.

### Import Local Images/Inputs

```shell
bento-bench prepare-local \
    --manifest manifest.json \
    --image /path/to/image \
    --input <input string> \ # Or --input-path to load from file
    --description "A local benchmark (1B)"
    --data-dir ./data
```
This will copy the provided image/input into the data dir, and append them to the manifest

## Uploading Suites

To tar and upload a manifest and data dir, run:

```shell
R2_ACCESS_KEY=<key> \
R2_SECRET_KEY=<secret-key \
./scripts/upload-suite.sh <suite-name>
```