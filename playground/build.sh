#!/bin/sh
# Build the wasm interpreter and place it next to index.html.
# Then serve the directory with any static file server, e.g.:
#   python3 -m http.server -d playground 8000
set -e
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown 2>/dev/null || true
cargo build --release --lib --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/ting.wasm playground/ting.wasm
echo "playground/ting.wasm ready"
