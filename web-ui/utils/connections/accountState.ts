import type { AccountInfo } from '@/types';

/**
 * Calculate receiver error/warning state based on connected sources
 *
 * @param receiver - The receiver account
 * @param connectedSources - Array of connected source accounts
 * @param content - Internationalized content for error messages
 * @returns Object with error state, warning state, and error message
 */
export function calculateReceiverState(
  receiver: AccountInfo,
  connectedSources: AccountInfo[],
  content: {
    allSourcesInactive: string;
    someSourcesInactive: string;
  }
): {
  hasError: boolean;
  hasWarning: boolean;
  errorMsg: string;
} {
  // Count active and inactive sources
  let activeCount = 0;
  let inactiveCount = 0;

  connectedSources.forEach((source) => {
    if (source.isEnabled && !source.hasError) {
      activeCount++;
    } else {
      inactiveCount++;
    }
  });

  // Determine receiver state based on source states
  if (inactiveCount > 0 && activeCount === 0) {
    // All sources are inactive - ERROR
    return {
      hasError: true,
      hasWarning: false,
      errorMsg: content.allSourcesInactive,
    };
  } else if (inactiveCount > 0 && activeCount > 0) {
    // Some sources are inactive - WARNING
    return {
      hasError: false,
      hasWarning: true,
      errorMsg: content.someSourcesInactive,
    };
  } else {
    // All sources are active - NORMAL
    return {
      hasError: false,
      hasWarning: false,
      errorMsg: '',
    };
  }
}

/**
 * Check if an account is currently active (enabled and without errors)
 *
 * @param account - The account to check
 * @returns True if the account is active
 */
export function isAccountActive(account: AccountInfo): boolean {
  return account.isEnabled && !account.hasError;
}

/**
 * Get the connection line color based on account state
 *
 * @param account - The account to get color for
 * @returns CSS color string
 */
export function getConnectionColor(account: AccountInfo): string {
  if (account.hasError) {
    return '#ef4444'; // red-500
  } else if (account.hasWarning) {
    return '#eab308'; // yellow-500
  } else if (account.isEnabled) {
    return '#22c55e'; // green-500
  } else {
    return '#d1d5db'; // gray-300
  }
}

/**
 * Get stroke dasharray for connection line based on account state
 *
 * @param account - The account to get dasharray for
 * @returns SVG stroke-dasharray value
 */
export function getConnectionDashArray(account: AccountInfo): string | undefined {
  const isActive = isAccountActive(account);
  return isActive ? undefined : '5,5';
}
