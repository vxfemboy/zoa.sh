#!/bin/bash

# Build script for WASM module

echo "Building WASM module..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Installing..."
    cargo install wasm-pack
fi

# Build the WASM module
wasm-pack build --target web --out-dir static/wasm --no-typescript

# Check if build was successful
if [ $? -eq 0 ]; then
    echo "WASM build successful! Output in static/wasm/"
else
    echo "WASM build failed!"
    exit 1
fi

