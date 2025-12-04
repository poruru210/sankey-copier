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

  const runtimeStatus = account.runtimeStatus;
  const isConnected = runtimeStatus === 2 || (runtimeStatus === undefined && account.isActive);
  const isWaiting = runtimeStatus === 1;

  const colorClass = account.hasWarning
    ? 'bg-yellow-500' // Auto-trading OFF
    : isConnected
    ? 'bg-green-500'  // Connected
    : isWaiting
    ? 'bg-amber-500'  // Waiting (runtime status 1)
    : 'bg-gray-300';  // Disabled or unknown

  return <div className={`${dimensionClasses} ${colorClass}`}></div>;
}
