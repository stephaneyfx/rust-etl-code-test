# Rust ETL Code Test

## Purpose

This command-line tool transforms a JSONL billing report into a CSV file containing the average rate
for each record, excluding records with an average rate greater than 30.

## Requirements

- Rust 1.87.0 or newer
- Internet connection for cargo to fetch dependencies

## Run

```sh
cargo run --release < sample.jsonl
```

## Run tests

```sh
cargo test
```

## Help

```sh
cargo run -- --help
```

## Notes

- Every error in the input is considered fatal. This could easily be changed if that is undesirable.
