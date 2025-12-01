# bento-bench Design

## Core Features
- Fetches inputs and images for requests that have been posted on-chain
    - Alternatively inputs and images can be provided from local sources
- Supports aggregating benchmarks (input + image) into suites of benchmarks
- Executes suites of benchmarks against bento clusters and reports the results

## Manifest

Each suite of benchmarks should have a corresponding manifest.

TODO: Should we include `cycles` as a field for entries here?

```json
{
    "description": "Some info about this bench suite",
    "entries": [
        {
            "description": "An on-chain request - 42M",
            "image": "34a5c9394fb2fd3298ece07c16ec2ed009f6029a360f90f4e93933b55e2184d4",
            "input": "8b7df143d91c716ecfa5fc1730022f6b421b05cedee8fd52b1fc65a96030ad52"
        },
        {
            "description": "A custom image - 42M",
            "image": "34a5c9394fb2fd3298ece07c16ec2ed009f6029a360f90f4e93933b55e2184d4",
            "input": "8b7df143d91c716ecfa5fc1730022f6b421b05cedee8fd52b1fc65a96030ad52"
        }
    ]
}
```

## Data Storage

All inputs and images are stored the data directory, with respective subdirectories `/inputs` and `/images`. Inputs are named using their sha256 hash, and all images according to their image ID.

## Prepare

### Option 1 - Request IDs

```shell
bento-bench prepare-request \
    --manifest manifest.json \
    --request-id 0x1234...ABCD \
    --description "A simple request (500M)" \
    --data-dir ./data
    --rpc-url "http://node:8545"
```
This will fetch the image and input for the provided request ID, store them in the data directory, and append a new entry to the manifest.

### Option 2 - Local Image/Input

```shell
bento-bench prepare-local \
    --manifest manifest.json \
    --image /path/to/image \
    --input <input string> \ # Or --input-path to load from file
    --description "A local benchmark (1B)"
    --data-dir ./data
```

> `--init <description>` can be specified to create a new manifest file with the provided description. Otherwise, `bench-bench prepare` will fail if the manifest does not exist.

## Run

```shell
bento-bench run \
    --manifest manifest.json
    --data-dir ./data
```
Example output:
```
TBD
```

> Optionally `--json` can be specified to output the results in json format (TBD) to be consumed by other tooling.

## In Practice

To use bento-bench in practice, a collection of benchmarks/suites need to identified. Each suite can be constructed as a distinct manifest with a corresponding data directory. Each suite (manifest + data dir) can be tarballed, and uploaded to an S3 bucket.

To run a suite, users can fetch the data from the S3 bucket, untar, and run `bento-bench run`.

> Note: Bash scripts will be included to perform uploading/downloading of suites.