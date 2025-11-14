import type { AccountInfo } from '@/types';

interface StatusIndicatorBarProps {
  account: AccountInfo;
  type: 'source' | 'receiver';
  isMobile?: boolean;
}

/**
 * Colored status indicator bar shown on the left (receiver) or right (source) of account cards
 * On mobile: shown on top (receiver) or bottom (source) as horizontal bars
 */
export function StatusIndicatorBar({ account, type, isMobile = false }: StatusIndicatorBarProps) {
  // Base dimensions: vertical (desktop) or horizontal (mobile)
  const dimensionClasses = isMobile ? 'h-1 w-full' : 'w-2 flex-shrink-0';

  // Only show for receivers on the left/top, sources on the right/bottom
  if (type === 'receiver') {
    return (
      <div
        className={`${dimensionClasses} ${
          account.hasError
            ? 'bg-red-500'
            : account.hasWarning
            ? 'bg-yellow-500'
            : account.isEnabled
            ? 'bg-green-500'
            : 'bg-gray-300'
        }`}
      ></div>
    );
  } else {
    return (
      <div
        className={`${dimensionClasses} ${
          account.hasError
            ? 'bg-red-500'
            : account.isEnabled && !account.hasError
            ? 'bg-green-500'
            : 'bg-gray-300'
        }`}
      ></div>
    );
  }
}
