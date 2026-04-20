# bento-bench

A utility for creating and running benchmarks against [Bento](https://docs.boundless.network/provers/bento) clusters.

## Installing

### Binaries

See [releases](https://github.com/zerokrab/bento-bench/releases).

### Build From Source
```shell
git clone github.com/zerokrab/bento-bench
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
| Signal                   | 50B     | 4     | https://boundless-benchmarks.mintybasil.dev/suites/suite-signal-4.tar.zst     |
| Kailua                   | 12B-17B | 4     | https://boundless-benchmarks.mintybasil.dev/suites/suite-kailua-4.tar.zst     |

> Note: Please open an issue if you would like to see other suites added.

To fetch and run a suite directly:
```shell
bento-bench run --fetch <link>
```

Example:
```shell
bento-bench run --fetch https://boundless-benchmarks.mintybasil.dev/suites/suite-og-4-1m-10m.tar.zst
```

Or download and extract manually:
```shell
curl <link> | tar -xv --zstd
bento-bench run --data-dir ./data
```

See `bento-bench run --help` for more configuration options.

### Docker

```shell
docker run --mount <data-path>:/data ghcr.io/zerokrab/bento-bench:latest run --data /data
```
### Timing Precision 
bento-bench can time the execution of proofs by either:
- Checking TaskDB for start/end times (`--check-taskdb`)
- Recording wall clock time

For the best accuracy, TaskDB should checked. If it is not checked, `--poll-interval` may need to be specified to get accurate readings for smaller proofs.


## Creating Benchmarks

### Data Directory Layout

```
data/
├── manifest.json         # Benchmark index
├── images/{image_id}.elf # RISC0 ELF binaries
└── inputs/{input_id}.bin # Serialized input blobs
```

### Manifest Structure

The data directory contains a `manifest.json` that describes each benchmark entry:

```json
{
  "description": "My benchmark suite",
  "entries": [
    {
      "description": "A simple request (500M)",
      "image_id": "abc123...",
      "input_id": "def456...",
      "cycles": 500000000
    }
  ]
}
```

`image_id` and `input_id` correspond to filenames under `data/images/` and `data/inputs/`. The `cycles` field is computed automatically during the prepare phase.

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

### Uploading Suites

To tar and upload a data dir to an R2/S3 bucket, run:

```shell
R2_ACCESS_KEY=<key> \
R2_SECRET_KEY=*** \
./scripts/upload-suite.sh <data-dir> <suite-name>
```

# License

This library is free software; you can redistribute it and/or modify it
under the terms of the GNU Lesser General Public License as published by
the Free Software Foundation.

See the [LICENSE](LICENSE) file or https://www.gnu.org/licenses/lgpl-3.0.html
for the full license text.
