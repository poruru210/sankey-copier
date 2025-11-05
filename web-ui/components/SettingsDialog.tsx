'use client';

import { useState, useEffect, useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import { SimpleAccountSelector } from '@/components/SimpleAccountSelector';
import { useSettingsValidation } from '@/hooks/useSettingsValidation';
import type { CopySettings, CreateSettingsRequest, EaConnection } from '@/types';
import { AlertCircle, AlertTriangle } from 'lucide-react';

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (data: CreateSettingsRequest | CopySettings) => void;
  initialData?: CopySettings | null;
  connections: EaConnection[];
  existingSettings: CopySettings[];
}

export function SettingsDialog({
  open,
  onOpenChange,
  onSave,
  initialData,
  connections,
  existingSettings
}: SettingsDialogProps) {
  const content = useIntlayer('settings-dialog');

  // Convert all validation messages to plain strings
  // Try accessing .value property or use String() constructor
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

  // No longer needed - render content directly with .value

  const [formData, setFormData] = useState({
    master_account: '',
    slave_account: '',
    lot_multiplier: 1.0,
    reverse_trade: false,
  });

  useEffect(() => {
    if (initialData) {
      setFormData({
        master_account: initialData.master_account,
        slave_account: initialData.slave_account,
        lot_multiplier: initialData.lot_multiplier || 1.0,
        reverse_trade: initialData.reverse_trade,
      });
    } else {
      setFormData({
        master_account: '',
        slave_account: '',
        lot_multiplier: 1.0,
        reverse_trade: false,
      });
    }
  }, [initialData, open]);

  // Validate form data
  const validation = useSettingsValidation({
    masterAccount: formData.master_account,
    slaveAccount: formData.slave_account,
    lotMultiplier: formData.lot_multiplier,
    existingSettings,
    connections,
    currentSettingId: initialData?.id,
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

    if (initialData) {
      onSave({
        ...initialData,
        ...formData,
      });
    } else {
      onSave(formData);
    }
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{initialData ? content.editTitle.value : content.createTitle.value}</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="space-y-6">
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
              <div className="rounded-md bg-red-50 dark:bg-red-950 p-4 border border-red-200 dark:border-red-800">
                <div className="flex">
                  <AlertCircle className="h-5 w-5 text-red-400 mr-2 flex-shrink-0 mt-0.5" />
                  <div className="flex-1">
                    <h3 className="text-sm font-medium text-red-800 dark:text-red-200">
                      {content.errorTitle.value}
                    </h3>
                    <ul className="mt-2 text-sm text-red-700 dark:text-red-300 list-disc list-inside space-y-1">
                      {validation.errors.map((error, index) => (
                        <li key={index}>{error}</li>
                      ))}
                    </ul>
                  </div>
                </div>
              </div>
            )}

            {validation.warnings.length > 0 && validation.errors.length === 0 && (
              <div className="rounded-md bg-yellow-50 dark:bg-yellow-950 p-4 border border-yellow-200 dark:border-yellow-800">
                <div className="flex">
                  <AlertTriangle className="h-5 w-5 text-yellow-400 mr-2 flex-shrink-0 mt-0.5" />
                  <div className="flex-1">
                    <h3 className="text-sm font-medium text-yellow-800 dark:text-yellow-200">
                      {content.warningTitle.value}
                    </h3>
                    <ul className="mt-2 text-sm text-yellow-700 dark:text-yellow-300 list-disc list-inside space-y-1">
                      {validation.warnings.map((warning, index) => (
                        <li key={index}>{warning}</li>
                      ))}
                    </ul>
                  </div>
                </div>
              </div>
            )}

            {/* Copy Settings Section */}
            <div className="space-y-4">
              <div className="space-y-1">
                <h3 className="text-sm font-medium flex items-center gap-2">
                  <span className="text-lg">⚙️</span>
                  {content.copySettingsLabel.value}
                </h3>
              </div>

              {/* Lot Multiplier */}
              <div>
                <Label htmlFor="lot_multiplier">
                  {content.lotMultiplier.value}
                  <span className="text-red-500 ml-1">*</span>
                </Label>
                <Input
                  id="lot_multiplier"
                  type="number"
                  step="0.01"
                  min="0.01"
                  max="100"
                  value={formData.lot_multiplier}
                  onChange={(e) => setFormData({ ...formData, lot_multiplier: parseFloat(e.target.value) || 1.0 })}
                  required
                />
                <p className="text-xs text-muted-foreground mt-1">
                  {content.lotMultiplierDescription.value}
                </p>
              </div>

              {/* Reverse Trade */}
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="reverse_trade"
                  checked={formData.reverse_trade}
                  onCheckedChange={(checked) =>
                    setFormData({ ...formData, reverse_trade: checked as boolean })
                  }
                />
                <Label htmlFor="reverse_trade" className="cursor-pointer">
                  {content.reverseTrade.value} - {content.reverseDescription.value}
                </Label>
              </div>
            </div>
          </div>

          <DialogFooter className="mt-6">
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              {content.cancel.value}
            </Button>
            <Button type="submit" disabled={!validation.isValid}>
              {initialData ? content.save.value : content.saveAndEnable.value}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
