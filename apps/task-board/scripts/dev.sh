#!/bin/bash
set -e
cd "$(dirname "$0")/.."
echo "Building task-board..."
cargo build --release --manifest-path rs/Cargo.toml
echo "Starting dev server on :3000"
./rs/target/release/task-board --demo public --port 3000
