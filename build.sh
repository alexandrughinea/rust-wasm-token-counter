#!/bin/bash
# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
  echo "wasm-pack not found, installing..."
  cargo install wasm-pack
fi

# Build the WASM package
echo "Building WASM package..."
wasm-pack build --target web

# Create dist directory if it doesn't exist
mkdir -p dist

# Copy files to dist directory
echo "Copying files to dist directory..."
cp -r pkg dist/
cp public/index.html dist/
cp public/worker.js dist/

echo "Build complete. Files are in the dist directory."
echo "You can serve the app with a local HTTP server, e.g.:"
echo "cd dist && python3 -m http.server 8080"