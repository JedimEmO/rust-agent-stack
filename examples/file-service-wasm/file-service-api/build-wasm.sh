#!/bin/bash

set -eu
# Build script for generating TypeScript bindings

echo "Building WASM package for web target..."
wasm-pack build --target web --out-dir pkg --features wasm-client

echo "Build complete! TypeScript bindings are in ./pkg/"
echo ""
echo "To use in TypeScript:"
echo "  1. Copy the pkg directory to your TypeScript project"
echo "  2. Import with: import init, { WasmDocumentServiceClient } from './pkg/file_service_api';"
echo "  3. Initialize WASM: await init();"
echo "  4. Create client: const client = new WasmDocumentServiceClient('http://localhost:3000');"
