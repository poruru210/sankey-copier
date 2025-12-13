'use client';

// SlaveSettingsDrawer - Nested drawer for viewing/editing slave settings
// Opens from MasterSettingsDrawer when user clicks on a slave in the list
// Allows editing of slave-specific copy settings

import { useState, useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import { useAtomValue } from 'jotai';
import { Drawer, DrawerContent, DrawerHeader, DrawerTitle, DrawerFooter } from '@/components/ui/drawer';
import { useMediaQuery } from '@/hooks/useMediaQuery';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { BrokerIcon } from '@/components/ui/BrokerIcon';
import { AlertCircle, CheckCircle, Loader2 } from 'lucide-react';
import { apiClientAtom } from '@/lib/atoms/site';
import { DRAWER_SIZE_SETTINGS } from '@/lib/ui-constants';
import { DrawerInfoCard } from '@/components/ui/drawer-section';
import { SlaveSettingsForm, type SlaveSettingsFormData } from '@/components/features/settings/SlaveSettingsForm';
import type { TradeGroupMember, SlaveSettings } from '@/types';

interface SlaveSettingsDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  member: TradeGroupMember | null;
  masterAccount: string;
  /** Callback when settings are saved successfully */
  onSaved?: () => void;
}

export function SlaveSettingsDrawer({
  open,
  onOpenChange,
  member,
  masterAccount,
  onSaved,
}: SlaveSettingsDrawerProps) {
  const content = useIntlayer('settings-dialog');
  const apiClient = useAtomValue(apiClientAtom);

  // Responsive: right drawer for desktop, bottom drawer for mobile
  const isDesktop = useMediaQuery('(min-width: 768px)');
  const side = isDesktop ? 'right' : 'bottom';

  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

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

  // Initialize form data when member changes
  useEffect(() => {
    if (member) {
      const settings = member.slave_settings;
      // Convert symbol_mappings array to comma-separated string
      const mappingsStr = settings.symbol_mappings
        ?.map(m => `${m.source_symbol}=${m.target_symbol}`)
        .join(',') || '';
      // Convert allowed_magic_numbers array to comma-separated string
      const magicStr = settings.filters?.allowed_magic_numbers
        ?.map(n => n.toString())
        .join(', ') || '';

      setFormData({
        lot_calculation_mode: settings.lot_calculation_mode || 'multiplier',
        lot_multiplier: settings.lot_multiplier || 1.0,
        reverse_trade: settings.reverse_trade,
        symbol_prefix: settings.symbol_prefix || '',
        symbol_suffix: settings.symbol_suffix || '',
        symbol_mappings: mappingsStr,
        source_lot_min: settings.source_lot_min ?? null,
        source_lot_max: settings.source_lot_max ?? null,
        // Open Sync Policy fields
        sync_mode: settings.sync_mode ?? 'skip',
        limit_order_expiry_min: settings.limit_order_expiry_min ?? null,
        market_sync_max_pips: settings.market_sync_max_pips ?? null,
        max_slippage: settings.max_slippage ?? null,
        copy_pending_orders: settings.copy_pending_orders ?? false,
        // Trade Execution fields
        max_retries: settings.max_retries ?? 3,
        max_signal_delay_ms: settings.max_signal_delay_ms ?? 5000,
        use_pending_order_for_delayed: settings.use_pending_order_for_delayed ?? false,
        // Filter fields
        allowed_magic_numbers: magicStr,
      });
      setMessage(null);
    }
  }, [member, open]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!member || !apiClient) return;

    setSaving(true);
    setMessage(null);

    try {
      // Convert comma-separated mappings back to array format
      const symbolMappings = formData.symbol_mappings
        ? formData.symbol_mappings.split(',').map(pair => {
            const [source, target] = pair.split('=').map(s => s.trim());
            return { source_symbol: source, target_symbol: target };
          }).filter(m => m.source_symbol && m.target_symbol)
        : [];

      // Convert comma-separated magic numbers to array format
      const allowedMagicNumbers = formData.allowed_magic_numbers
        ? formData.allowed_magic_numbers.split(',')
            .map(s => parseInt(s.trim(), 10))
            .filter(n => !isNaN(n))
        : null;

      const settings: SlaveSettings = {
        lot_calculation_mode: formData.lot_calculation_mode,
        lot_multiplier: formData.lot_multiplier,
        reverse_trade: formData.reverse_trade,
        symbol_prefix: formData.symbol_prefix || null,
        symbol_suffix: formData.symbol_suffix || null,
        symbol_mappings: symbolMappings,
        filters: {
          ...member.slave_settings.filters,
          allowed_magic_numbers: allowedMagicNumbers && allowedMagicNumbers.length > 0 ? allowedMagicNumbers : null,
        },
        config_version: member.slave_settings.config_version,
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
      };

      await apiClient.updateTradeGroupMember(masterAccount, member.slave_account, settings);
      setMessage({ type: 'success', text: content.settingsSavedSuccess.value });

      // Notify parent and close after short delay
      setTimeout(() => {
        onSaved?.();
        onOpenChange(false);
      }, 1000);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : content.settingsSaveFailed.value;
      setMessage({ type: 'error', text: errorMessage });
      console.error('Error updating slave settings:', err);
    } finally {
      setSaving(false);
    }
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

  const masterInfo = splitAccountName(masterAccount);
  const slaveInfo = member ? splitAccountName(member.slave_account) : { brokerName: '', accountNumber: '' };

  return (
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
                  <BrokerIcon brokerName={masterInfo.brokerName} size="sm" className="flex-shrink-0" />
                  <div className="min-w-0">
                    <div className="font-medium text-sm truncate">
                      {masterInfo.brokerName}
                    </div>
                    {masterInfo.accountNumber && (
                      <div className="text-xs text-muted-foreground">
                        {masterInfo.accountNumber}
                      </div>
                    )}
                  </div>
                </div>

                {/* Arrow */}
                <span className="text-muted-foreground text-xl">→</span>

                {/* Slave Account */}
                <div className="flex items-center gap-2 min-w-0">
                  <BrokerIcon brokerName={slaveInfo.brokerName} size="sm" className="flex-shrink-0" />
                  <div className="min-w-0">
                    <div className="font-medium text-sm truncate">
                      {slaveInfo.brokerName}
                    </div>
                    {slaveInfo.accountNumber && (
                      <div className="text-xs text-muted-foreground">
                        {slaveInfo.accountNumber}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            </DrawerInfoCard>

            {/* Slave Settings Form */}
            <SlaveSettingsForm
              formData={formData}
              onChange={setFormData}
              disabled={saving}
            />

            {/* Message Display */}
            {message && (
              <div
                className={cn(
                  'px-4 py-3 rounded-lg flex items-center gap-2 text-sm',
                  message.type === 'success'
                    ? 'bg-green-500/10 border border-green-500 text-green-600 dark:text-green-400'
                    : 'bg-destructive/10 border border-destructive text-destructive'
                )}
              >
                {message.type === 'success' ? (
                  <CheckCircle className="h-4 w-4" />
                ) : (
                  <AlertCircle className="h-4 w-4" />
                )}
                {message.text}
              </div>
            )}
          </div>

          <DrawerFooter className="flex-shrink-0 pt-4 border-t mt-4">
            <div className="flex w-full justify-end items-center gap-2">
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                {content.cancel.value}
              </Button>
              <Button type="submit" disabled={saving}>
                {saving ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin mr-2" />
                    {content.saving.value}
                  </>
                ) : (
                  content.save.value
                )}
              </Button>
            </div>
          </DrawerFooter>
        </form>
      </DrawerContent>
    </Drawer>
  );
}
