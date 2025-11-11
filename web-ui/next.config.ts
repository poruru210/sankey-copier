import type { NextConfig } from 'next';
import { withIntlayer } from 'next-intlayer/server';

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
