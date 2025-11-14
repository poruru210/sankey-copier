import type { NextConfig } from 'next';
import { withIntlayer } from 'next-intlayer/server';

// Static export mode for Tauri - no Node.js runtime required
// Assets are served directly by Tauri's webview
const isProd = process.env.NODE_ENV === 'production';
const internalHost = process.env.TAURI_DEV_HOST || 'localhost';

const nextConfig: NextConfig = {
  // Static export mode - pre-renders all pages to HTML/CSS/JS at build time
  output: 'export',

  // Required for static export - Next.js Image Optimization API not available
  images: {
    unoptimized: true,
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
