/**
 * Site represents a remote SANKEY Copier server instance
 */
export interface Site {
  /** Unique identifier (UUID) */
  id: string;
  /** Display name for the site */
  name: string;
  /** Base URL of the Rust server API (e.g., "http://localhost:3000") */
  siteUrl: string;
}

/**
 * Default local site configuration
 */
export const DEFAULT_SITE: Site = {
  id: 'local',
  name: 'Local',
  siteUrl: 'https://localhost:3000',
};

/**
 * LocalStorage keys for site management
 */
export const STORAGE_KEYS = {
  SITES: 'sankey-copier-sites',
  SELECTED_SITE_ID: 'sankey-copier-selected-site-id',
} as const;
