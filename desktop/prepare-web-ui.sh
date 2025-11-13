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

# Read project name from package.json
PROJECT_NAME=$(node -pe "require('./package.json').name")
echo "Project name: $PROJECT_NAME"

# Copy static files to the correct location within standalone structure
# Next.js standalone expects: <project-name>/.next/static (preserving project structure)
echo "Copying static files..."
mkdir -p "$BUNDLE_DIR/$PROJECT_NAME/.next"
cp -r .next/static "$BUNDLE_DIR/$PROJECT_NAME/.next/"

# Copy public directory to the correct location
if [ -d "public" ]; then
  echo "Copying public directory..."
  cp -r public "$BUNDLE_DIR/$PROJECT_NAME/"
fi

echo "Web UI bundle prepared successfully at $BUNDLE_DIR"
