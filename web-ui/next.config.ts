import type { NextConfig } from 'next';
import { withIntlayer } from 'next-intlayer/server';

// Rust Server API URL - configurable via environment variable
// Default: http://localhost:3000 for production
// This allows the installer to configure the API endpoint dynamically
const apiBaseUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';

const nextConfig: NextConfig = {
  // Output standalone for Windows service deployment
  output: 'standalone',

  // Exclude unnecessary packages from standalone output
  // Reduces bundle size and eliminates dev dependencies
  outputFileTracingExcludes: {
    '*': [
      // Build tools (not needed at runtime)
      'esbuild',
      'webpack',
      '@swc/core',
      'typescript',

      // Testing tools
      '@playwright/test',
      '@types/*',

      // Linting and formatting
      'eslint',
      'eslint-config-next',
      'prettier',

      // PostCSS and Tailwind build tools
      'postcss',
      'autoprefixer',
      'tailwindcss',
    ],
  },

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
