#!/bin/bash

cargo fmt --all -- --check
cargo clippy --all-targets --all-features
cargo test --all --verbose
cargo build --all --verbose