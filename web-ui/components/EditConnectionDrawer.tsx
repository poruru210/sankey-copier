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
import { BrokerIcon } from '@/components/BrokerIcon';
import { SlaveSettingsForm, type SlaveSettingsFormData } from '@/components/SlaveSettingsForm';
import { DRAWER_SIZE_SETTINGS } from '@/lib/ui-constants';
import { DrawerInfoCard } from '@/components/ui/drawer-section';
import type { CopySettings } from '@/types';

interface EditConnectionDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (data: CopySettings) => void;
  onDelete: (data: CopySettings) => void;
  setting: CopySettings;
}

export function EditConnectionDrawer({
  open,
  onOpenChange,
  onSave,
  onDelete,
  setting
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
  });

  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  // Initialize form data when setting changes
  useEffect(() => {
    if (setting) {
      setFormData({
        lot_calculation_mode: setting.lot_calculation_mode || 'multiplier',
        lot_multiplier: setting.lot_multiplier || 1.0,
        reverse_trade: setting.reverse_trade,
        symbol_prefix: setting.symbol_prefix || '',
        symbol_suffix: setting.symbol_suffix || '',
        symbol_mappings: setting.symbol_map || '',
        source_lot_min: setting.source_lot_min ?? null,
        source_lot_max: setting.source_lot_max ?? null,
      });
    }
  }, [setting, open]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    onSave({
      ...setting,
      lot_calculation_mode: formData.lot_calculation_mode,
      lot_multiplier: formData.lot_multiplier,
      reverse_trade: formData.reverse_trade,
      symbol_prefix: formData.symbol_prefix || undefined,
      symbol_suffix: formData.symbol_suffix || undefined,
      symbol_map: formData.symbol_mappings || undefined,
      source_lot_min: formData.source_lot_min,
      source_lot_max: formData.source_lot_max,
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

              {/* Slave Settings Form (shared component) */}
              <SlaveSettingsForm
                formData={formData}
                onChange={setFormData}
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
