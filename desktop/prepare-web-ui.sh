#!/bin/bash
set -e

echo "Preparing web-ui for Tauri bundling..."

# Navigate to web-ui directory
cd "$(dirname "$0")/../web-ui"

# Build Next.js standalone
echo "Building Next.js standalone..."
pnpm install --frozen-lockfile
pnpm build

# Create bundle directory
BUNDLE_DIR="../desktop/web-ui"
rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR"

# Copy standalone build
echo "Copying standalone build..."
cp -r .next/standalone/* "$BUNDLE_DIR/"

# Copy static files (Next.js standalone uses flat structure)
echo "Copying static files..."
mkdir -p "$BUNDLE_DIR/.next"
cp -r .next/static "$BUNDLE_DIR/.next/"

# Copy public directory
if [ -d "public" ]; then
  echo "Copying public directory..."
  cp -r public "$BUNDLE_DIR/"
fi

echo "Web UI bundle prepared successfully at $BUNDLE_DIR"
