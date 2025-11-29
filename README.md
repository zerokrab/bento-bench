# Bento Utils

## Preparing a manifest

This will take your manifest, fetch the image and inputs if `image_id` isn't specified.
Create a new directory in the project root called data. Then make a file called `manifest.json`. Here is an example:

```json
{
    "notes": "Some info about this bench suite",
    "entries": [
        {
            "image_id": "34a5c9394fb2fd3298ece07c16ec2ed009f6029a360f90f4e93933b55e2184d4",
            "request_id": "0xe198c6944cae382902a375b0b8673084270a7f8e56e8883e",
            "description": "42M",
            "label": "onchain-order-generator"
        },
        {
            "image_id": "34a5c9394fb2fd3298ece07c16ec2ed009f6029a360f90f4e93933b55e2184d4",
            "request_id": "0xc197ebe12c7bcf1d9f3b415342bdbc795425335c11a49e11",
            "description": "347M",
            "label": "offchain-order-generator"
        },
        {
            "image_id": "34a5c9394fb2fd3298ece07c16ec2ed009f6029a360f90f4e93933b55e2184d4",
            "request_id": "0xc197ebe12c7bcf1d9f3b415342bdbc795425335c8ad8091d",
            "description": "147M",
            "label": "offchain-order-generator"
        }
    ]
}
```

Then run `cargo run --  datasheet prepare`. In addition to downloading the images and inputs, it will also save this information to sqlite so that reports can be generated.

A UUID will be generated for this manifest and stored into the db. Also a `manifest-<UUID>.json` will be made too with information liek the image\_id filled in if you didn't do so before.

## Benching against a Bento Cluster

To bench against Bento, run `cargo run --  datasheet bench`. Every run of this command will be uniquely identified by a UUID.

The output of this is a file called `datasheet-<UUID>.json` as well, it is saved into the database, so you can actually get rid of all json files when theyre not needed as long as you keep the database.

## Misc

* `cargo run --  datasheet db` - view manifests and datasheets that are saved in the db.

## Docker

Run a single benchmark:

```shell
docker run --entrypoint=/app/bencher ghcr.io/2boys1proof/bencher bench -f benchmark.elf -i benchmark.input
```

Run all benchmarks in a directory:

```shell
docker run --mount type=bind,src=<host_dir>,dst=/data ghcr.io/2boys1proof/bencher
```
