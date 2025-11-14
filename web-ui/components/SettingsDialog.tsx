'use client';

import { useState, useEffect, useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
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
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import { SimpleAccountSelector } from '@/components/SimpleAccountSelector';
import { BrokerIcon } from '@/components/BrokerIcon';
import { useSettingsValidation } from '@/hooks/useSettingsValidation';
import type { CopySettings, CreateSettingsRequest, EaConnection } from '@/types';
import { AlertCircle, AlertTriangle } from 'lucide-react';

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (data: CreateSettingsRequest | CopySettings) => void;
  onDelete?: (data: CopySettings) => void;
  initialData?: CopySettings | null;
  connections: EaConnection[];
  existingSettings: CopySettings[];
}

export function SettingsDialog({
  open,
  onOpenChange,
  onSave,
  onDelete,
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

  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

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
      onSave({
        ...formData,
        enabled: false,
      });
    }
    onOpenChange(false);
  };

  const handleDelete = () => {
    if (initialData && onDelete) {
      onDelete(initialData);
      onOpenChange(false);
      setShowDeleteConfirm(false);
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

  return (
    <>
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{initialData ? content.editTitle.value : content.createTitle.value}</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="flex flex-col overflow-hidden">
          <div className="space-y-4 overflow-y-auto pr-2">
            {/* Account Selection - Only show in create mode */}
            {!initialData && (
              <>
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
              </>
            )}

            {/* Account Display - Only show in edit mode */}
            {initialData && (() => {
              const masterAccount = splitAccountName(formData.master_account);
              const slaveAccount = splitAccountName(formData.slave_account);

              return (
                <div className="space-y-3">
                  <div className="space-y-1">
                    <h3 className="text-sm font-medium flex items-center gap-2">
                      <span className="text-lg">🔗</span>
                      {content.connectionLabel.value}
                    </h3>
                  </div>
                  <div className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                    <div className="flex items-center gap-3">
                      {/* Master Account */}
                      <div className="flex items-center gap-2 flex-1">
                        <BrokerIcon brokerName={masterAccount.brokerName} size="sm" />
                        <div className="flex-1">
                          <div className="font-medium text-sm text-gray-900 dark:text-gray-100">
                            {masterAccount.brokerName}
                          </div>
                          {masterAccount.accountNumber && (
                            <div className="text-xs text-gray-600 dark:text-gray-400">
                              {masterAccount.accountNumber}
                            </div>
                          )}
                        </div>
                      </div>

                      {/* Arrow */}
                      <span className="text-gray-400 text-lg">→</span>

                      {/* Slave Account */}
                      <div className="flex items-center gap-2 flex-1">
                        <BrokerIcon brokerName={slaveAccount.brokerName} size="sm" />
                        <div className="flex-1">
                          <div className="font-medium text-sm text-gray-900 dark:text-gray-100">
                            {slaveAccount.brokerName}
                          </div>
                          {slaveAccount.accountNumber && (
                            <div className="text-xs text-gray-600 dark:text-gray-400">
                              {slaveAccount.accountNumber}
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                    <p className="text-xs text-muted-foreground mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
                      {content.connectionDescription.value}
                    </p>
                  </div>
                </div>
              );
            })()}

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

          <DialogFooter className="mt-6 flex-shrink-0 pt-4 border-t">
            <div className="flex w-full justify-between items-center">
              <div>
                {initialData && onDelete && (
                  <Button type="button" variant="destructive" onClick={() => setShowDeleteConfirm(true)}>
                    {content.delete.value}
                  </Button>
                )}
              </div>
              <div className="flex gap-2">
                <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                  {content.cancel.value}
                </Button>
                <Button type="submit" disabled={!validation.isValid}>
                  {initialData ? content.save.value : content.saveAndEnable.value}
                </Button>
              </div>
            </div>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>

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
