import { defineConfig } from '@playwright/test';
import { spawnSync } from 'node:child_process';

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

const externalBaseUrl = process.env.PLAYWRIGHT_BASE_URL;
const shouldStartServer = !externalBaseUrl;
const requestedPort = process.env.PLAYWRIGHT_PORT
  ? Number(process.env.PLAYWRIGHT_PORT)
  : undefined;
const allocatedPort = shouldStartServer
  ? requestedPort ?? findAvailablePortSync()
  : undefined;
const resolvedBaseUrl = externalBaseUrl ?? `http://127.0.0.1:${allocatedPort ?? 8080}`;
// Ensure helper utilities can read the same base URL
process.env.PLAYWRIGHT_BASE_URL = resolvedBaseUrl;
if (shouldStartServer) {
  process.env.NEXT_ASSET_PREFIX = resolvedBaseUrl;
}
const envAssignments = shouldStartServer
  ? {
    PORT: String(allocatedPort),
    NEXT_ASSET_PREFIX: resolvedBaseUrl,
  }
  : undefined;

const envPrefix = shouldStartServer
  ? process.platform === 'win32'
    ? `${Object.entries(envAssignments!)
      .map(([key, value]) => `set ${key}=${value}`)
      .join('&&')}&&`
    : Object.entries(envAssignments!)
      .map(([key, value]) => `${key}=${value}`)
      .join(' ')
  : '';

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
      command: `${envPrefix} bun run next dev --hostname 0.0.0.0 --port ${allocatedPort}`.trim(),
      url: resolvedBaseUrl,
      reuseExistingServer: false,
      timeout: 120 * 1000,
    }
    : undefined,
});

export default config;
