# Bento Utils

## From Source
Fetching an image and input from order:

```
RUST_LOG=info cargo run fetch --request-id 0xf353bda16a83399c11e09615ee7ac326a5a08ccf98b02453  -f kailua-14B
```

Running against Bento

```
RUST_BACKTRACE=1  RUST_LOG="info,bencher=debug" cargo run bench -f kailua-14B.elf -i kailua-14B.input
```

## Docker

Run a single benchmark:
```shell
docker run --entrypoint=/app/bencher ghcr.io/2boys1proof/bencher bench -f benchmark.elf -i benchmark.input
```

Run all benchmarks in a directory:
```shell
docker run --mount type=bind,src=<host_dir>,dst=/data ghcr.io/2boys1proof/bencher
```