import type { NextConfig } from 'next';
import { withIntlayer } from 'next-intlayer/server';

// Build mode selection via environment variable
// - 'export': For Tauri desktop app - static HTML/CSS/JS only
// - undefined (default): For Vercel deployment - standard Next.js SSR/SSG
const buildMode = process.env.NEXT_BUILD_MODE;
const isProd = process.env.NODE_ENV === 'production';
const internalHost = process.env.TAURI_DEV_HOST || 'localhost';

const nextConfig: NextConfig = {
  // Output mode: export for Tauri desktop app, default for Vercel
  output: buildMode === 'export' ? 'export' : undefined,

  // Image optimization disabled for export mode only
  images: {
    unoptimized: buildMode === 'export',
    remotePatterns: [
      {
        protocol: 'https',
        hostname: 'www.google.com',
      },
    ],
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
