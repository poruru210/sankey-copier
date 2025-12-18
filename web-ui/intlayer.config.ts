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
  editor: {
    /**
     * Required
     * The URL of the application.
     * This is the URL that the visual editor will target.
     * Example: 'http://localhost:3000'
     */
    applicationURL: 'http://localhost:8080',
    /**
     * Optional
     * Default to `true`. If `false`, the editor is disabled and cannot be accessed.
     * Can be used to disable the editor in specific environments, like production, for security reasons.
     */
    enabled: process.env.NODE_ENV === 'development',
    /**
     * Optional
     * Default to `8000`.
     * The port of the editor server.
     */
    port: 8000,
    /**
     * Optional
     * Default to "http://localhost:8000"
     * The URL of the editor server.
     */
    editorURL: 'http://localhost:8000',
  },
};

export default config;
