#!/usr/bin/env sh
set -e

# From apps/extension

# Clean previous build
rm -rf build
mkdir build

# Check if bun is installed
if ! command -v bun >/dev/null 2>&1; then
  echo "Error: 'bun' is not installed. Please install Bun (https://bun.sh) to build the extension." >&2
  exit 127
fi

# Build the extension (from the extension root)
bun run build

# Copy content CSS to dist/content/
mkdir -p dist/content
cp content/injectBadge.css dist/content/injectBadge.css

# Copy necessary files to build/
cp manifest.json build/
cp -r popup build/
cp -r dist build/

# Build the extension zip using web-ext from inside build/
cd build
bunx web-ext build --source-dir=./ --artifacts-dir=../dist
cd ..

# Clean up build directory
rm -rf build