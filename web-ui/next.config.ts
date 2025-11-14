import type { NextConfig } from 'next';
import { withIntlayer } from 'next-intlayer/server';

// Build mode selection via environment variable
// - 'standalone': For Windows service (installer) - includes Node.js runtime
// - 'export': For Tauri desktop app - static HTML/CSS/JS only (default)
const buildMode = process.env.NEXT_BUILD_MODE || 'export';
const isProd = process.env.NODE_ENV === 'production';
const internalHost = process.env.TAURI_DEV_HOST || 'localhost';

const nextConfig: NextConfig = {
  // Output mode: standalone for server, export for Tauri
  output: buildMode === 'standalone' ? 'standalone' : 'export',

  // Image optimization disabled for export mode, auto-configured for standalone
  images: {
    unoptimized: buildMode === 'export',
  },

  // For Tauri dev mode - use localhost:8080, for production use relative paths
  assetPrefix: isProd ? undefined : `http://${internalHost}:8080`,

  webpack: (config) => {
    // Filter out problematic environment variables
    const env = { ...process.env };
    Object.keys(env).forEach((key) => {
      if (key.includes(' ') || key.includes('(') || key.includes(')')) {
        delete env[key];
      }
    });
    return config;
  },
};

export default withIntlayer(nextConfig);
