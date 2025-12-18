'use client';

// EditConnectionDrawer - Drawer component for editing existing copy settings
// Uses SlaveSettingsForm for consistent UX with CreateConnectionDialog
// Replaces EditCopySettingsDialog with a Drawer-based approach

import { useState, useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Drawer, DrawerContent, DrawerHeader, DrawerTitle, DrawerFooter } from '@/components/ui/drawer';
import { useMediaQuery } from '@/hooks/useMediaQuery';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { BrokerIcon } from '@/components/ui/BrokerIcon';
import { SlaveSettingsForm, type SlaveSettingsFormData } from '@/components/features/settings/SlaveSettingsForm';
import { DRAWER_SIZE_SETTINGS } from '@/lib/ui-constants';
import { DrawerInfoCard } from '@/components/ui/drawer-section';
import type { CopySettings, EaConnection } from '@/types';

interface EditConnectionDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (data: CopySettings) => void;
  onDelete: (data: CopySettings) => void;
  setting: CopySettings;
  connection?: EaConnection;
}

export function EditConnectionDrawer({
  open,
  onOpenChange,
  onSave,
  onDelete,
  setting,
  connection
}: EditConnectionDrawerProps) {
  const content = useIntlayer('settings-dialog');

  // Responsive: right drawer for desktop, bottom drawer for mobile
  const isDesktop = useMediaQuery('(min-width: 768px)');
  const side = isDesktop ? 'right' : 'bottom';

  const [formData, setFormData] = useState<SlaveSettingsFormData>({
    lot_calculation_mode: 'multiplier',
    lot_multiplier: 1.0,
    reverse_trade: false,
    symbol_prefix: '',
    symbol_suffix: '',
    symbol_mappings: '',
    source_lot_min: null,
    source_lot_max: null,
    // Open Sync Policy defaults
    sync_mode: 'skip',
    limit_order_expiry_min: null,
    market_sync_max_pips: null,
    max_slippage: null,
    copy_pending_orders: false,
    // Trade Execution defaults
    max_retries: 3,
    max_signal_delay_ms: 5000,
    use_pending_order_for_delayed: false,
    // Filter defaults
    allowed_magic_numbers: '',
  });

  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  // Initialize form data when setting changes
  useEffect(() => {
    if (setting) {
      // Convert symbol_mappings array to string format for editing
      // Prefer array format, fallback to string format
      const symbolMappingsString = setting.symbol_mappings && setting.symbol_mappings.length > 0
        ? setting.symbol_mappings.map(m => `${m.source_symbol}=${m.target_symbol}`).join(',')
        : (setting.symbol_map || '');

      // Convert allowed_magic_numbers array to comma-separated string
      const magicStr = setting.filters?.allowed_magic_numbers
        ?.map(n => n.toString())
        .join(', ') || '';

      setFormData({
        lot_calculation_mode: setting.lot_calculation_mode || 'multiplier',
        lot_multiplier: setting.lot_multiplier || 1.0,
        reverse_trade: setting.reverse_trade,
        symbol_prefix: setting.symbol_prefix || '',
        symbol_suffix: setting.symbol_suffix || '',
        symbol_mappings: symbolMappingsString,
        source_lot_min: setting.source_lot_min ?? null,
        source_lot_max: setting.source_lot_max ?? null,
        // Open Sync Policy fields
        sync_mode: setting.sync_mode ?? 'skip',
        limit_order_expiry_min: setting.limit_order_expiry_min ?? null,
        market_sync_max_pips: setting.market_sync_max_pips ?? null,
        max_slippage: setting.max_slippage ?? null,
        copy_pending_orders: setting.copy_pending_orders ?? false,
        // Trade Execution fields
        max_retries: setting.max_retries ?? 3,
        max_signal_delay_ms: setting.max_signal_delay_ms ?? 5000,
        use_pending_order_for_delayed: setting.use_pending_order_for_delayed ?? false,
        // Filter fields
        allowed_magic_numbers: magicStr,
      });
    }
  }, [setting, open]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    // Parse symbol_mappings string to array format
    // Format: "XAUUSD=GOLD,EURUSD=EUR" -> [{source_symbol: "XAUUSD", target_symbol: "GOLD"}, ...]
    const symbolMappingsArray = formData.symbol_mappings
      ? formData.symbol_mappings.split(',').filter(s => s.trim()).map(pair => {
        const [source, target] = pair.split('=').map(s => s.trim());
        return { source_symbol: source || '', target_symbol: target || '' };
      }).filter(m => m.source_symbol && m.target_symbol)
      : [];

    // Convert comma-separated magic numbers to array format
    const allowedMagicNumbers = formData.allowed_magic_numbers
      ? formData.allowed_magic_numbers.split(',')
        .map(s => parseInt(s.trim(), 10))
        .filter(n => !isNaN(n))
      : null;

    onSave({
      ...setting,
      lot_calculation_mode: formData.lot_calculation_mode,
      lot_multiplier: formData.lot_multiplier,
      reverse_trade: formData.reverse_trade,
      symbol_prefix: formData.symbol_prefix || undefined,
      symbol_suffix: formData.symbol_suffix || undefined,
      symbol_mappings: symbolMappingsArray,
      symbol_map: formData.symbol_mappings || undefined,
      source_lot_min: formData.source_lot_min,
      source_lot_max: formData.source_lot_max,
      // Open Sync Policy fields
      sync_mode: formData.sync_mode,
      limit_order_expiry_min: formData.limit_order_expiry_min,
      market_sync_max_pips: formData.market_sync_max_pips,
      max_slippage: formData.max_slippage,
      copy_pending_orders: formData.copy_pending_orders,
      // Trade Execution fields
      max_retries: formData.max_retries,
      max_signal_delay_ms: formData.max_signal_delay_ms,
      use_pending_order_for_delayed: formData.use_pending_order_for_delayed,
      // Filter fields
      filters: {
        ...setting.filters,
        allowed_magic_numbers: allowedMagicNumbers && allowedMagicNumbers.length > 0 ? allowedMagicNumbers : null,
      },
    });
    onOpenChange(false);
  };

  const handleDelete = () => {
    onDelete(setting);
    onOpenChange(false);
    setShowDeleteConfirm(false);
  };

  // Split account name into broker name and account number
  const splitAccountName = (accountName: string) => {
    const lastUnderscoreIndex = accountName.lastIndexOf('_');
    if (lastUnderscoreIndex === -1) {
      return { brokerName: accountName, accountNumber: '' };
    }
    return {
      brokerName: accountName.substring(0, lastUnderscoreIndex).replace(/_/g, ' '),
      accountNumber: accountName.substring(lastUnderscoreIndex + 1),
    };
  };

  const masterAccount = splitAccountName(setting.master_account);
  const slaveAccount = splitAccountName(setting.slave_account);

  return (
    <>
      <Drawer open={open} onOpenChange={onOpenChange} direction={side}>
        <DrawerContent
          side={side}
          className={cn(
            'overflow-hidden p-6',
            isDesktop
              ? `h-full w-full ${DRAWER_SIZE_SETTINGS.desktop}`
              : DRAWER_SIZE_SETTINGS.mobile
          )}
        >
          <DrawerHeader className={isDesktop ? 'mt-0' : ''}>
            <DrawerTitle>{content.editTitle.value}</DrawerTitle>
          </DrawerHeader>

          <form onSubmit={handleSubmit} className="flex flex-col flex-1 overflow-hidden">
            <div className="flex-1 overflow-y-auto pr-2 space-y-6">
              {/* Connection Display */}
              <DrawerInfoCard>
                <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-4">
                  {/* Master Account */}
                  <div className="flex items-center gap-2 min-w-0">
                    <BrokerIcon brokerName={masterAccount.brokerName} size="sm" className="flex-shrink-0" />
                    <div className="min-w-0">
                      <div className="font-medium text-sm truncate">
                        {masterAccount.brokerName}
                      </div>
                      {masterAccount.accountNumber && (
                        <div className="text-xs text-muted-foreground">
                          {masterAccount.accountNumber}
                        </div>
                      )}
                    </div>
                  </div>

                  {/* Arrow */}
                  <span className="text-muted-foreground text-xl">→</span>

                  {/* Slave Account */}
                  <div className="flex items-center gap-2 min-w-0">
                    <BrokerIcon brokerName={slaveAccount.brokerName} size="sm" className="flex-shrink-0" />
                    <div className="min-w-0">
                      <div className="font-medium text-sm truncate">
                        {slaveAccount.brokerName}
                      </div>
                      {slaveAccount.accountNumber && (
                        <div className="text-xs text-muted-foreground">
                          {slaveAccount.accountNumber}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              </DrawerInfoCard>

              <SlaveSettingsForm
                formData={formData}
                onChange={setFormData}
                detectedContext={connection?.symbol_context}
              />
            </div>

            <DrawerFooter className="flex-shrink-0 pt-4 border-t mt-4">
              <div className="flex w-full justify-between items-center">
                <div>
                  <Button type="button" variant="destructive" onClick={() => setShowDeleteConfirm(true)}>
                    {content.delete.value}
                  </Button>
                </div>
                <div className="flex gap-2">
                  <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                    {content.cancel.value}
                  </Button>
                  <Button type="submit">
                    {content.save.value}
                  </Button>
                </div>
              </div>
            </DrawerFooter>
          </form>
        </DrawerContent>
      </Drawer>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{content.deleteConfirmTitle.value}</AlertDialogTitle>
            <AlertDialogDescription>
              {content.deleteConfirmDescription.value}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{content.cancel.value}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {content.delete.value}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
