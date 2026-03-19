#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "Building Docker image..."
docker build -t sgleam-desktop-builder desktop/

echo "Building sgleam-desktop..."
docker run --rm -v "$(pwd)":/src sgleam-desktop-builder

echo "Binary: target/release/sgleam-desktop"
ls -lh target/release/sgleam-desktop
ldd target/release/sgleam-desktop
