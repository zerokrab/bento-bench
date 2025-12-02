# bento-bench

## Installing

### Binaries

WIP

### Build from source
```shell
git clone github.com/2boys1proof/bento-bench
cd bento-bench
cargo build --release
```


## Running Benchmarks

> WIP - suites will be provided soon.

Once you have a manifest and data directory, benchmarks can be run with
```shell
bento-bench run \
    --manifest manifest.json \
    --data-dir ./data
```

To configure the bento backend, see `bento-bench run --help`.

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

## Docker

```shell
docker run --mount <data-path>:/data <manifest-path>:/manifest.json ghcr.io/2boys1proof/bento-bench 
```

