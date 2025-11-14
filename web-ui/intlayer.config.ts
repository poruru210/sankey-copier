import { type IntlayerConfig } from 'intlayer';

const config: IntlayerConfig = {
  internationalization: {
    locales: ['en', 'ja'],
    defaultLocale: 'en',
  },
  routing: {
    // Use 'prefix-all' to ensure both en and ja locales have URL prefixes
    // This prevents routing conflicts where /en/connections would match [locale]/page.tsx
    mode: 'prefix-all',
  },
};

export default config;
