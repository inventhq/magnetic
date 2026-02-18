#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "Building infinite-scroll server..."
cargo build --release --manifest-path rs/Cargo.toml

ITEMS=${1:-1500}
PORT=${2:-3001}

echo "Starting with $ITEMS items on :$PORT"
./rs/target/release/infinite-scroll --public public --port "$PORT" --items "$ITEMS"
