# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```shell
# Build
cargo build --release

# Run clippy (linter)
cargo clippy

# Run tests
cargo test

# Run a single test by name
cargo test <test_name>

# Build only the CLI binary
cargo build --release -p bento-bench
```

## Architecture Overview

This is a Rust workspace with three crates:

- **`crates/bento-bench`** — CLI binary with three subcommands: `prepare-request`, `prepare-local`, and `run`
- **`crates/core`** — Shared types (`IterReq` enum, `PANIC_STR`) used by both the host and guest programs
- **`crates/guests`** — RISC0 guest programs compiled to ELF, embedded via `risc0-build` at compile time

### Data Flow

**Prepare phase** (`prepare-request` or `prepare-local`): fetches or copies an ELF image and input blob into a local `data/` directory, computes cycles via `risc0_zkvm`, and appends a `ManifestEntry` to `data/manifest.json`.

**Run phase** (`run`): reads `manifest.json`, and for each entry calls `prove_stark()` (and optionally `prove_snark()`) against a [Bento](https://docs.boundless.network/provers/bento) cluster. Results are collected into `BenchResult` structs and printed as tables; optionally saved as JSON.

### Prover Module (`crates/bento-bench/src/prover/`)

- `prove_stark()` — uploads ELF + input to Bento, creates a session (with `exec_only` flag for first pass), polls for completion, returns `SessionStats` and timing. Optionally queries PostgreSQL (`taskdb`) for precise job durations if `DATABASE_URL` is set (default: `postgresql://worker:password@localhost:5432/taskdb`).
- `prove_snark()` — takes the `SessionId` from a completed STARK proof, submits a SNARK request, and polls for completion.

### Data Directory Layout

```
data/
├── manifest.json              # Describes each benchmark entry
├── images/{image_id}.elf      # RISC0 ELF programs
└── inputs/{input_id}.input    # Serialized input blobs
```

### Guest Programs

- **`bento-sample`** — multi-case guest driven by `IterReq` enum: simple loops, composition proofs, keccak hashing, and combinations. `IterReq::Iter(0)` triggers a panic (for failure testing).
- **`ordergen-loop`** — minimal loop that spins until a target cycle count is reached; used for predictable cycle benchmarks.

### Key Environment Variables

| Variable | Default | Purpose |
|---|---|---|
| `BENTO_API_URL` | `http://localhost:8081` | Bento cluster endpoint |
| `BENTO_API_KEY` | — | API key for Bento |
| `DATABASE_URL` | `postgresql://worker:password@localhost:5432/taskdb` | Optional PostgreSQL for precise timing |
| `RUST_LOG` | `info` | Log level |
| `RPC_URL` | — | RPC endpoint for `prepare-request` |
