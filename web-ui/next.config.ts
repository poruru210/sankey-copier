import type { NextConfig } from 'next';
import { withIntlayer } from 'next-intlayer/server';

// Rust Server API URL - configurable via environment variable
// Default: http://localhost:8080 for production
// This allows the installer to configure the API endpoint dynamically
const apiBaseUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

const nextConfig: NextConfig = {
  // Output standalone for Windows service deployment
  output: 'standalone',

  // Proxy API calls to Rust Server
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: `${apiBaseUrl}/api/:path*`,
      },
      {
        source: '/ws',
        destination: `${apiBaseUrl}/ws`,
      },
    ];
  },
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
