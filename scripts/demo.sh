#!/bin/bash
# Build and run the Magnetic server-driven UI demo locally.
set -e

cd "$(dirname "$0")/.."

echo "[1/4] Building WASM reducer..."
cargo build --target wasm32-unknown-unknown --release \
  --manifest-path rs/crates/magnetic-reducer/Cargo.toml 2>&1 | tail -1

echo "[2/4] Building dev server..."
cargo build --release \
  --manifest-path rs/crates/magnetic-dev-server/Cargo.toml 2>&1 | tail -1

echo "[3/4] Building client runtime..."
node js/packages/sdk-web-runtime/build.cjs
cp js/packages/sdk-web-runtime/dist/magnetic.min.js demo/magnetic.js

echo "[4/4] Starting server on http://localhost:3000"
exec ./rs/crates/magnetic-dev-server/target/release/magnetic-dev-server \
  --demo demo \
  --wasm rs/crates/magnetic-reducer/target/wasm32-unknown-unknown/release/magnetic_reducer.wasm \
  --port 3000
