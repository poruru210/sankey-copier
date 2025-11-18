import type { AccountInfo } from '@/types';

interface StatusIndicatorBarProps {
  account: AccountInfo;
  type: 'source' | 'receiver';
  isMobile?: boolean;
}

/**
 * Colored status indicator bar shown on the left (receiver) or right (source) of account cards
 * On mobile: shown on top (receiver) or bottom (source) as horizontal bars
 *
 * Color logic (simplified):
 * - Active (isActive=true) → Green
 * - Auto-trading OFF (hasWarning=true) → Yellow
 * - Disabled (isEnabled=false) → Gray
 */
export function StatusIndicatorBar({ account, type, isMobile = false }: StatusIndicatorBarProps) {
  // Base dimensions: vertical (desktop) or horizontal (mobile)
  const dimensionClasses = isMobile ? 'h-1 w-full' : 'w-2 flex-shrink-0';

  // Determine color based on account state (same logic for both source and receiver)
  const colorClass = account.hasWarning
    ? 'bg-yellow-500'  // Auto-trading OFF
    : account.isActive
    ? 'bg-green-500'   // Active (ready for trading)
    : 'bg-gray-300';   // Inactive or disabled

  return <div className={`${dimensionClasses} ${colorClass}`}></div>;
}
