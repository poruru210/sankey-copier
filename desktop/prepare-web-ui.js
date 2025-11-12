#!/usr/bin/env node
/**
 * Prepare web-ui for Tauri bundling
 * This script builds Next.js standalone and copies it to desktop/web-ui/
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const isWindows = process.platform === 'win32';

function log(message, color = 'cyan') {
  const colors = {
    cyan: '\x1b[36m',
    yellow: '\x1b[33m',
    green: '\x1b[32m',
    reset: '\x1b[0m'
  };
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function exec(command, cwd) {
  log(`Running: ${command}`, 'yellow');
  execSync(command, { cwd, stdio: 'inherit', shell: isWindows ? 'powershell.exe' : '/bin/bash' });
}

function copyRecursiveSync(src, dest) {
  if (!fs.existsSync(src)) {
    throw new Error(`Source directory not found: ${src}`);
  }

  const stats = fs.statSync(src);
  if (stats.isDirectory()) {
    if (!fs.existsSync(dest)) {
      fs.mkdirSync(dest, { recursive: true });
    }
    const entries = fs.readdirSync(src);
    for (const entry of entries) {
      copyRecursiveSync(path.join(src, entry), path.join(dest, entry));
    }
  } else {
    fs.copyFileSync(src, dest);
  }
}

function rmRecursiveSync(dir) {
  if (fs.existsSync(dir)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

try {
  log('Preparing web-ui for Tauri bundling...');

  // Navigate to web-ui directory
  const rootDir = path.resolve(__dirname, '..');
  const webUiDir = path.join(rootDir, 'web-ui');
  const bundleDir = path.join(__dirname, 'web-ui');

  log(`Web UI directory: ${webUiDir}`);
  log(`Bundle directory: ${bundleDir}`);

  // Build Next.js standalone
  log('Building Next.js standalone...', 'yellow');
  exec('pnpm install --frozen-lockfile', webUiDir);
  exec('pnpm build', webUiDir);

  // Create bundle directory
  log('Creating bundle directory...', 'yellow');
  rmRecursiveSync(bundleDir);
  fs.mkdirSync(bundleDir, { recursive: true });

  // Copy standalone build
  log('Copying standalone build...', 'yellow');
  const standaloneSrc = path.join(webUiDir, '.next', 'standalone');
  if (!fs.existsSync(standaloneSrc)) {
    throw new Error(`Standalone build not found at: ${standaloneSrc}`);
  }
  copyRecursiveSync(standaloneSrc, bundleDir);

  // Copy static files
  log('Copying static files...', 'yellow');
  const staticSrc = path.join(webUiDir, '.next', 'static');
  const staticDest = path.join(bundleDir, '.next', 'static');
  fs.mkdirSync(path.dirname(staticDest), { recursive: true });
  copyRecursiveSync(staticSrc, staticDest);

  // Copy public directory
  const publicSrc = path.join(webUiDir, 'public');
  if (fs.existsSync(publicSrc)) {
    log('Copying public directory...', 'yellow');
    const publicDest = path.join(bundleDir, 'public');
    copyRecursiveSync(publicSrc, publicDest);
  }

  log(`Web UI bundle prepared successfully at ${bundleDir}`, 'green');
} catch (error) {
  console.error('Error preparing web-ui:', error.message);
  process.exit(1);
}
