#!/bin/bash

set -e

echo "Building WGPU Fractals for Web..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack is not installed. Please install it with:"
    echo "curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
fi

# Build the project
echo "Building with wasm-pack..."
wasm-pack build --target web --out-dir pkg --dev

echo "Build complete!"
echo ""
echo "To serve the application:"
echo "1. Start a local web server in this directory:"
echo "   python3 -m http.server 8000"
echo "   # or"
echo "   npx serve ."
echo ""
echo "2. Open http://localhost:8000 in your browser"
echo ""
echo "Note: You need to serve the files over HTTP due to WASM security requirements."
echo "The application won't work if you open index.html directly in the browser."
