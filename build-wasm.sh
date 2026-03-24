#!/bin/bash

set -eu
cd "$(dirname "$0")"

# release
cargo build --release \
  --target wasm32-unknown-unknown --lib \
  --no-default-features --features wasm
wasm-pack build --target bundler --no-default-features --features wasm

# debug (optional)
if [ "${NO_DEBUG:-false}" != "true" ]; then
  cargo build \
    --target wasm32-unknown-unknown --lib \
    --no-default-features --features wasm
  wasm-pack build --dev --out-dir pkg-dev --target bundler --no-default-features --features wasm
fi
