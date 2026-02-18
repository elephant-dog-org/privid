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

# Generate contract types
bun run typechain

# Build the extension (from the extension root)
bun run build

# Copy content CSS to dist/content/
mkdir -p dist/content
cp content/injectBadge.css dist/content/injectBadge.css

# Copy Gmail content CSS to dist/content/gmail/
mkdir -p dist/content/gmail
cp content/gmail/injectGmailBadge.css dist/content/gmail/injectGmailBadge.css

# Copy Twitter content CSS to dist/content/twitter/
mkdir -p dist/content/twitter
cp content/twitter/injectTwitterBadge.css dist/content/twitter/injectTwitterBadge.css

# Background service worker is built to dist/background/ by vite

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