'use client';

import { useIntlayer } from 'next-intlayer';
import { Button } from '@/components/ui/button';

interface FilterIndicatorProps {
  selectedMaster: string;
  selectedMasterName: string | null | undefined;
  onClearFilter: () => void;
}

export function FilterIndicator({
  selectedMaster,
  selectedMasterName,
  onClearFilter,
}: FilterIndicatorProps) {
  const sidebarContent = useIntlayer('master-account-sidebar');

  if (selectedMaster === 'all' || !selectedMasterName) {
    return null;
  }

  // Split account name into broker and account number
  const lastUnderscoreIndex = selectedMasterName.lastIndexOf('_');
  const brokerName = lastUnderscoreIndex === -1
    ? selectedMasterName
    : selectedMasterName.substring(0, lastUnderscoreIndex).replace(/_/g, ' ');
  const accountNumber = lastUnderscoreIndex === -1
    ? ''
    : selectedMasterName.substring(lastUnderscoreIndex + 1);

  return (
    <div
      className="mb-4 flex items-center justify-between px-4 py-2 bg-accent rounded-lg border border-border animate-in fade-in slide-in-from-top-2 duration-300"
      data-testid="master-filter-indicator"
    >
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium">{sidebarContent.viewingAccount}:</span>
        <div className="flex flex-col">
          <span className="text-sm text-muted-foreground font-medium">{brokerName}</span>
          {accountNumber && (
            <span className="text-xs text-muted-foreground">{accountNumber}</span>
          )}
        </div>
      </div>
      <Button
        variant="ghost"
        size="sm"
        onClick={onClearFilter}
        className="h-auto px-2 py-1"
      >
        {sidebarContent.clearFilter}
      </Button>
    </div>
  );
}
