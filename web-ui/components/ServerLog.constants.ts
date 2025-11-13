// ServerLog component constants

export const LOG_VIEWER_CONSTANTS = {
  // Heights
  DEFAULT_HEIGHT: 350,
  MIN_HEIGHT: 200,
  COLLAPSED_BAR_HEIGHT: 40,

  // Viewport ratios
  MAX_HEIGHT_RATIO: 0.9, // 90% of viewport height

  // Refresh intervals
  AUTO_REFRESH_INTERVAL_MS: 3000, // 3 seconds

  // Z-index layers
  Z_INDEX: 50,
} as const;

// Log level colors adapted for both light and dark modes
export const LOG_LEVEL_COLORS = {
  ERROR: 'text-red-600 dark:text-red-400',
  WARN: 'text-yellow-600 dark:text-yellow-400',
  INFO: 'text-blue-600 dark:text-blue-400',
  DEBUG: 'text-muted-foreground',
  DEFAULT: 'text-muted-foreground',
} as const;

export const DOM_SELECTORS = {
  MAIN_CONTENT: '.relative.z-10',
  PAGE_CONTAINER: '.min-h-screen',
} as const;
