#!/usr/bin/env bash
set -e

echo "Building WASM plugins for Sprawl..."
cd plugins
cargo component build --release

echo "Copying plugins to ~/.sprawl/plugins (if you want to use them locally)..."
mkdir -p ~/.sprawl/plugins
cp target/wasm32-wasip1/release/*_detector.wasm ~/.sprawl/plugins/

echo "Build complete."
