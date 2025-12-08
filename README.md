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

| Source                   | Cycles  | Count | Link                                                                          |   
|--------------------------|---------|-------|-------------------------------------------------------------------------------|
| Order Generator (Tiny)   | 1M-10M  | 4     | https://boundless-benchmarks.mintybasil.dev/suites/suite-og-4-1m-10m.tar.zst  |
| Order Generator (Small)  | 100M-1B | 5     | https://boundless-benchmarks.mintybasil.dev/suites/suite-og-4-100m-1b.tar.zst |
| Order Generator (Medium) | 1B      | 4     | https://boundless-benchmarks.mintybasil.dev/suites/suite-og-4-1b.tar.zst      |
| Order Generator (Large)  | 4B      | 4     | https://boundless-benchmarks.mintybasil.dev/suites/suite-og-4-4b.tar.zst      |
| Order Generator (Varied) | 50M-4B  | 5     | https://boundless-benchmarks.mintybasil.dev/suites/suite-og-4-50m-4b.tar.zst  |

o fetch and untar:
```shell
curl <link> | tar -xv --zstd
```

Once you have a data directory, benchmarks can be run with
```shell
bento-bench run \
    --data-dir ./data
```

To configure the bento backend, see `bento-bench run --help`. The summary of the benchmarks can be outputted to a json file with `--json <path>`.

## Docker

```shell
docker run --mount <data-path>:/data ghcr.io/2boys1proof/bento-bench 
```


## Creating Benchmarks

> Note: On first run, if no manifest exists one will be created with an empty description.

### Fetching Market Requests

```shell
bento-bench prepare-request \
    --request-id 0x1234...ABCD \
    --description "A simple request (500M)" \
    --data-dir ./data
    --rpc-url "http://node:8545"
```

This will fetch and save the image and input to the data dir, and append them to the manifest.

### Import Local Images/Inputs

```shell
bento-bench prepare-local \
    --image /path/to/image \
    --input <input string> \ # Or --input-path to load from file
    --description "A local benchmark (1B)"
    --data-dir ./data
```
This will copy the provided image/input into the data dir, and append them to the manifest.

## Uploading Suites

To tar and upload a data dir to an R2/S3 bucket, run:

```shell
R2_ACCESS_KEY=<key> \
R2_SECRET_KEY=<secret-key \
./scripts/upload-suite.sh <data-dir> <suite-name>
```