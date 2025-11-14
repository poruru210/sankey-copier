'use client';

import { useState, useEffect, useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { SimpleAccountSelector } from '@/components/SimpleAccountSelector';
import { useSettingsValidation } from '@/hooks/useSettingsValidation';
import type { CreateSettingsRequest, EaConnection, CopySettings } from '@/types';
import { AlertCircle, AlertTriangle } from 'lucide-react';

interface CreateConnectionDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreate: (data: CreateSettingsRequest) => void;
  connections: EaConnection[];
  existingSettings: CopySettings[];
}

export function CreateConnectionDialog({
  open,
  onOpenChange,
  onCreate,
  connections,
  existingSettings
}: CreateConnectionDialogProps) {
  const content = useIntlayer('settings-dialog');

  // Convert all validation messages to plain strings
  const validationMessages = useMemo(() => ({
    selectMasterAccount: String(content.validationSelectMasterAccount?.value ?? content.validationSelectMasterAccount),
    selectSlaveAccount: String(content.validationSelectSlaveAccount?.value ?? content.validationSelectSlaveAccount),
    sameAccountError: String(content.validationSameAccountError?.value ?? content.validationSameAccountError),
    lotMultiplierPositive: String(content.validationLotMultiplierPositive?.value ?? content.validationLotMultiplierPositive),
    lotMultiplierTooSmall: String(content.validationLotMultiplierTooSmall?.value ?? content.validationLotMultiplierTooSmall),
    lotMultiplierTooLarge: String(content.validationLotMultiplierTooLarge?.value ?? content.validationLotMultiplierTooLarge),
    duplicateSettings: String(content.validationDuplicateSettings?.value ?? content.validationDuplicateSettings),
    statusEnabled: String(content.validationStatusEnabled?.value ?? content.validationStatusEnabled),
    statusDisabled: String(content.validationStatusDisabled?.value ?? content.validationStatusDisabled),
    accountOffline: String(content.validationAccountOffline?.value ?? content.validationAccountOffline),
    accountTimeout: String(content.validationAccountTimeout?.value ?? content.validationAccountTimeout),
    accountNotInList: String(content.validationAccountNotInList?.value ?? content.validationAccountNotInList),
    circularReference: String(content.validationCircularReference?.value ?? content.validationCircularReference),
  }), [content]);

  const [formData, setFormData] = useState({
    master_account: '',
    slave_account: '',
  });

  useEffect(() => {
    if (open) {
      setFormData({
        master_account: '',
        slave_account: '',
      });
    }
  }, [open]);

  // Validate form data
  const validation = useSettingsValidation({
    masterAccount: formData.master_account,
    slaveAccount: formData.slave_account,
    lotMultiplier: 1.0,
    existingSettings,
    connections,
    currentSettingId: undefined,
    messages: validationMessages,
  });

  const handleMasterChange = (value: string) => {
    setFormData({ ...formData, master_account: value });
  };

  const handleSlaveChange = (value: string) => {
    setFormData({ ...formData, slave_account: value });
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    // Don't submit if validation fails
    if (!validation.isValid) {
      return;
    }

    onCreate({
      master_account: formData.master_account,
      slave_account: formData.slave_account,
      lot_multiplier: 1.0,
      reverse_trade: false,
      enabled: false,
    });
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{content.createTitle.value}</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="flex flex-col overflow-hidden">
          <div className="space-y-4 overflow-y-auto pr-2">
            {/* Master Account Selection */}
            <div className="space-y-3">
              <div className="space-y-1">
                <h3 className="text-sm font-medium flex items-center gap-2">
                  <span className="text-lg">📤</span>
                  {content.masterAccountLabel.value}
                </h3>
                <p className="text-xs text-muted-foreground">
                  {content.masterAccountDescription.value}
                </p>
              </div>
              <SimpleAccountSelector
                label=""
                value={formData.master_account}
                onChange={handleMasterChange}
                connections={connections}
                filterType="Master"
                required
              />
            </div>

            {/* Slave Account Selection */}
            <div className="space-y-3">
              <div className="space-y-1">
                <h3 className="text-sm font-medium flex items-center gap-2">
                  <span className="text-lg">📥</span>
                  {content.slaveAccountLabel.value}
                </h3>
                <p className="text-xs text-muted-foreground">
                  {content.slaveAccountDescription.value}
                </p>
              </div>
              <SimpleAccountSelector
                label=""
                value={formData.slave_account}
                onChange={handleSlaveChange}
                connections={connections}
                filterType="Slave"
                required
              />
            </div>

            {/* Validation Messages */}
            {validation.errors.length > 0 && (
              <div className="rounded-md bg-red-50 dark:bg-red-950 p-3 border border-red-200 dark:border-red-800">
                <div className="flex">
                  <AlertCircle className="h-4 w-4 text-red-400 mr-2 flex-shrink-0 mt-0.5" />
                  <div className="flex-1">
                    <h3 className="text-xs font-medium text-red-800 dark:text-red-200">
                      {content.errorTitle.value}
                    </h3>
                    <ul className="mt-1 text-xs text-red-700 dark:text-red-300 list-disc list-inside space-y-0.5">
                      {validation.errors.map((error, index) => (
                        <li key={index}>{error}</li>
                      ))}
                    </ul>
                  </div>
                </div>
              </div>
            )}

            {validation.warnings.length > 0 && validation.errors.length === 0 && (
              <div className="rounded-md bg-yellow-50 dark:bg-yellow-950 p-3 border border-yellow-200 dark:border-yellow-800">
                <div className="flex">
                  <AlertTriangle className="h-4 w-4 text-yellow-400 mr-2 flex-shrink-0 mt-0.5" />
                  <div className="flex-1">
                    <h3 className="text-xs font-medium text-yellow-800 dark:text-yellow-200">
                      {content.warningTitle.value}
                    </h3>
                    <ul className="mt-1 text-xs text-yellow-700 dark:text-yellow-300 list-disc list-inside space-y-0.5">
                      {validation.warnings.map((warning, index) => (
                        <li key={index}>{warning}</li>
                      ))}
                    </ul>
                  </div>
                </div>
              </div>
            )}
          </div>

          <DialogFooter className="mt-6 flex-shrink-0 pt-4 border-t">
            <div className="flex gap-2 justify-end w-full">
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                {content.cancel.value}
              </Button>
              <Button type="submit" disabled={!validation.isValid}>
                {content.saveAndEnable.value}
              </Button>
            </div>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
