import { defineConfig } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import { createConnection } from 'node:net';

function findAvailablePortSync(): number {
  const script = `
    const { createServer } = require('node:net');
    const server = createServer();
    server.unref();
    server.on('error', (error) => {
      console.error(error);
      process.exit(1);
    });
    server.listen(0, () => {
      const address = server.address();
      if (typeof address === 'object' && address) {
        process.stdout.write(String(address.port));
      } else {
        console.error('Could not determine free port');
        process.exit(1);
      }
      server.close(() => process.exit(0));
    });
  `;

  const result = spawnSync(process.execPath, ['-e', script], {
    encoding: 'utf-8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    const stderr = result.stderr?.toString() ?? 'unknown error';
    throw new Error(`Failed to allocate port: ${stderr}`);
  }

  const output = result.stdout?.toString().trim();
  const port = Number(output);

  if (!Number.isInteger(port)) {
    throw new Error(`Port allocation returned unexpected value: ${output || 'empty output'}`);
  }

  return port;
}

// Check if a port is already in use (dev server running)
function isPortInUseSync(port: number): boolean {
  const script = `
    const net = require('node:net');
    const socket = new net.Socket();
    socket.setTimeout(500);
    socket.on('connect', () => { socket.destroy(); process.stdout.write('1'); process.exit(0); });
    socket.on('timeout', () => { socket.destroy(); process.stdout.write('0'); process.exit(0); });
    socket.on('error', () => { process.stdout.write('0'); process.exit(0); });
    socket.connect(${port}, '127.0.0.1');
  `;
  const result = spawnSync(process.execPath, ['-e', script], {
    encoding: 'utf-8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  return result.stdout?.toString().trim() === '1';
}

const DEFAULT_PORT = 8080;
const externalBaseUrl = process.env.PLAYWRIGHT_BASE_URL;

// Check if dev server is already running on default port
const devServerRunning = !externalBaseUrl && isPortInUseSync(DEFAULT_PORT);

const shouldStartServer = !externalBaseUrl && !devServerRunning;
const requestedPort = process.env.PLAYWRIGHT_PORT
  ? Number(process.env.PLAYWRIGHT_PORT)
  : undefined;
const allocatedPort = shouldStartServer
  ? requestedPort ?? findAvailablePortSync()
  : DEFAULT_PORT;
const resolvedBaseUrl = externalBaseUrl ?? `http://127.0.0.1:${allocatedPort}`;
// Ensure helper utilities can read the same base URL
process.env.PLAYWRIGHT_BASE_URL = resolvedBaseUrl;
if (shouldStartServer) {
  process.env.NEXT_ASSET_PREFIX = resolvedBaseUrl;
}
const config = defineConfig({
  testDir: '__tests__',
  testMatch: /.*\.spec\.ts$/, // limit to E2E specs only
  fullyParallel: true,
  retries: process.env.CI ? 1 : 0,
  expect: {
    timeout: 10000,
  },
  use: {
    baseURL: resolvedBaseUrl,
    headless: true,
    trace: 'on-first-retry',
  },
  webServer: shouldStartServer
    ? {
      command: `bun run next dev --hostname 0.0.0.0 --port ${allocatedPort}`,
      url: resolvedBaseUrl,
      reuseExistingServer: true,
      timeout: 120 * 1000,
      env: {
        ...process.env,
        PORT: String(allocatedPort),
        NEXT_ASSET_PREFIX: resolvedBaseUrl,
      },
    }
    : undefined,
});

export default config;
