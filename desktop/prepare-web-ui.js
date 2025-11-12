#!/usr/bin/env node
/**
 * Prepare web-ui for Tauri bundling
 * This script builds Next.js standalone and copies it to desktop/web-ui/
 *
 * Features:
 * - Smart caching: Only rebuilds if source files changed
 * - Use --force to bypass cache and force rebuild
 * - Use --skip-install to skip pnpm install (faster when deps unchanged)
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const isWindows = process.platform === 'win32';
const args = process.argv.slice(2);
const forceRebuild = args.includes('--force');
const skipInstall = args.includes('--skip-install');

function log(message, color = 'cyan') {
  const colors = {
    cyan: '\x1b[36m',
    yellow: '\x1b[33m',
    green: '\x1b[32m',
    blue: '\x1b[34m',
    reset: '\x1b[0m'
  };
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function exec(command, cwd) {
  log(`Running: ${command}`, 'yellow');
  execSync(command, { cwd, stdio: 'inherit', shell: isWindows ? 'powershell.exe' : '/bin/bash' });
}

function getFileHash(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }
  const content = fs.readFileSync(filePath);
  return crypto.createHash('md5').update(content).digest('hex');
}

function getDirectoryMtime(dir, extensions = []) {
  if (!fs.existsSync(dir)) {
    return 0;
  }

  let maxMtime = 0;

  function walkDir(currentPath) {
    const entries = fs.readdirSync(currentPath, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(currentPath, entry.name);

      // Skip node_modules and .next
      if (entry.isDirectory() && (entry.name === 'node_modules' || entry.name === '.next')) {
        continue;
      }

      if (entry.isDirectory()) {
        walkDir(fullPath);
      } else {
        // Check file extension if filter is provided
        if (extensions.length === 0 || extensions.some(ext => entry.name.endsWith(ext))) {
          const stats = fs.statSync(fullPath);
          maxMtime = Math.max(maxMtime, stats.mtimeMs);
        }
      }
    }
  }

  walkDir(dir);
  return maxMtime;
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

function shouldRebuild(webUiDir, bundleDir, cacheFile) {
  // Always rebuild if --force flag is used
  if (forceRebuild) {
    log('Force rebuild requested (--force)', 'blue');
    return true;
  }

  // Check if bundle directory exists
  if (!fs.existsSync(bundleDir)) {
    log('Bundle directory does not exist, building...', 'blue');
    return true;
  }

  // Check if cache file exists
  if (!fs.existsSync(cacheFile)) {
    log('No cache file found, building...', 'blue');
    return true;
  }

  try {
    const cache = JSON.parse(fs.readFileSync(cacheFile, 'utf8'));

    // Check package.json hash
    const packageJsonPath = path.join(webUiDir, 'package.json');
    const packageJsonHash = getFileHash(packageJsonPath);
    if (packageJsonHash !== cache.packageJsonHash) {
      log('package.json changed, rebuilding...', 'blue');
      return true;
    }

    // Check pnpm-lock.yaml hash
    const lockfilePath = path.join(webUiDir, 'pnpm-lock.yaml');
    const lockfileHash = getFileHash(lockfilePath);
    if (lockfileHash !== cache.lockfileHash) {
      log('pnpm-lock.yaml changed, rebuilding...', 'blue');
      return true;
    }

    // Check source files modification time
    const appDir = path.join(webUiDir, 'app');
    const componentsDir = path.join(webUiDir, 'components');
    const libDir = path.join(webUiDir, 'lib');

    const sourceExtensions = ['.ts', '.tsx', '.js', '.jsx', '.css', '.json'];
    const currentMtime = Math.max(
      getDirectoryMtime(appDir, sourceExtensions),
      getDirectoryMtime(componentsDir, sourceExtensions),
      getDirectoryMtime(libDir, sourceExtensions)
    );

    if (currentMtime > cache.sourceMtime) {
      log('Source files changed, rebuilding...', 'blue');
      return true;
    }

    // Check next.config.ts
    const nextConfigPath = path.join(webUiDir, 'next.config.ts');
    const nextConfigHash = getFileHash(nextConfigPath);
    if (nextConfigHash !== cache.nextConfigHash) {
      log('next.config.ts changed, rebuilding...', 'blue');
      return true;
    }

    log('No changes detected, using cached build ✓', 'green');
    return false;
  } catch (error) {
    log(`Error reading cache: ${error.message}, rebuilding...`, 'blue');
    return true;
  }
}

function saveBuildCache(webUiDir, cacheFile) {
  const packageJsonPath = path.join(webUiDir, 'package.json');
  const lockfilePath = path.join(webUiDir, 'pnpm-lock.yaml');
  const nextConfigPath = path.join(webUiDir, 'next.config.ts');

  const appDir = path.join(webUiDir, 'app');
  const componentsDir = path.join(webUiDir, 'components');
  const libDir = path.join(webUiDir, 'lib');

  const sourceExtensions = ['.ts', '.tsx', '.js', '.jsx', '.css', '.json'];
  const sourceMtime = Math.max(
    getDirectoryMtime(appDir, sourceExtensions),
    getDirectoryMtime(componentsDir, sourceExtensions),
    getDirectoryMtime(libDir, sourceExtensions)
  );

  const cache = {
    packageJsonHash: getFileHash(packageJsonPath),
    lockfileHash: getFileHash(lockfilePath),
    nextConfigHash: getFileHash(nextConfigPath),
    sourceMtime: sourceMtime,
    timestamp: new Date().toISOString()
  };

  fs.writeFileSync(cacheFile, JSON.stringify(cache, null, 2));
  log('Build cache updated', 'green');
}

try {
  log('Preparing web-ui for Tauri bundling...');

  // Navigate to web-ui directory
  const rootDir = path.resolve(__dirname, '..');
  const webUiDir = path.join(rootDir, 'web-ui');
  const bundleDir = path.join(__dirname, 'web-ui');
  const cacheFile = path.join(__dirname, '.build-cache.json');

  log(`Web UI directory: ${webUiDir}`);
  log(`Bundle directory: ${bundleDir}`);

  // Check if rebuild is needed
  if (!shouldRebuild(webUiDir, bundleDir, cacheFile)) {
    log('Using cached web-ui build (no changes detected)', 'green');
    log('Use --force to force rebuild', 'blue');
    process.exit(0);
  }

  // Build Next.js standalone
  log('Building Next.js standalone...', 'yellow');

  if (!skipInstall) {
    exec('pnpm install --frozen-lockfile', webUiDir);
  } else {
    log('Skipping pnpm install (--skip-install)', 'blue');
  }

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

  // Save build cache
  saveBuildCache(webUiDir, cacheFile);

  log(`Web UI bundle prepared successfully at ${bundleDir}`, 'green');
} catch (error) {
  console.error('Error preparing web-ui:', error.message);
  process.exit(1);
}
