import type { NextConfig } from 'next';
import { withIntlayer } from 'next-intlayer/server';

const nextConfig: NextConfig = {
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: 'http://127.0.0.1:8080/api/:path*',
      },
      {
        source: '/ws',
        destination: 'http://127.0.0.1:8080/ws',
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
