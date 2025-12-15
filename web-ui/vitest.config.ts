import { defineConfig } from 'vitest/config';
import path from 'node:path';

export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './vitest.setup.ts',
    include: ['__tests__/components/**/*.test.*', 'lib/**/*.test.*'],
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname),
    },
  },
});
