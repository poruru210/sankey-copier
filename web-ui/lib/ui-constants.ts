// UI Constants - Shared styling constants for consistent UI across the application

/**
 * Standard drawer size for all drawers (unified)
 * - Desktop: Right-side drawer with max-w-2xl (672px)
 * - Mobile: Bottom drawer with 92vh height
 *
 * Used by: CreateConnectionDialog, EditConnectionDrawer, MasterSettingsDrawer, etc.
 */
export const DRAWER_SIZE_SETTINGS = {
  desktop: 'max-w-2xl',
  mobile: 'h-[92vh]',
} as const;
