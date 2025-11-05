import type { NextConfig } from 'next';
import { withIntlayer } from 'next-intlayer/server';

const nextConfig: NextConfig = {
  // Allow external network access during development
  // Specify the actual IP address of your network interface
  allowedDevOrigins: [
    'http://10.5.0.2:5173',
    'http://localhost:5173',
    '10.5.0.2:5173',
    '10.5.0.2',
  ],
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
