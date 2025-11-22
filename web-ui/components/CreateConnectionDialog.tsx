'use client';

import { useState, useEffect, useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { SimpleAccountSelector } from '@/components/SimpleAccountSelector';
import { useSettingsValidation } from '@/hooks/useSettingsValidation';
import type { CreateSettingsRequest, EaConnection, CopySettings } from '@/types';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
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
  const [step, setStep] = useState(1);

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
    lot_multiplier: 1.0,
    reverse_trade: false,
    symbol_prefix: '',
    symbol_suffix: '',
    symbol_mappings: '',
  });

  useEffect(() => {
    if (open) {
      setFormData({
        master_account: '',
        slave_account: '',
        lot_multiplier: 1.0,
        reverse_trade: false,
        symbol_prefix: '',
        symbol_suffix: '',
        symbol_mappings: '',
      });
      setStep(1);
    }
  }, [open]);

  // Validate form data
  const validation = useSettingsValidation({
    masterAccount: formData.master_account,
    slaveAccount: formData.slave_account,
    lotMultiplier: formData.lot_multiplier,
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

    // If we are in step 1, just move to step 2
    if (step === 1) {
      if (isStep1Valid) {
        setStep(2);
      }
      return;
    }

    // Don't submit if validation fails
    if (!validation.isValid) {
      return;
    }

    onCreate({
      master_account: formData.master_account,
      slave_account: formData.slave_account,
      lot_multiplier: formData.lot_multiplier,
      reverse_trade: formData.reverse_trade,
      status: 0, // STATUS_DISABLED (default OFF)
      symbol_prefix: formData.symbol_prefix || undefined,
      symbol_suffix: formData.symbol_suffix || undefined,
      symbol_mappings: formData.symbol_mappings || undefined,
    });
    onOpenChange(false);
  };

  // Check if step 1 is valid to proceed
  const isStep1Valid = useMemo(() => {
    // Basic fields must be filled
    if (!formData.master_account || !formData.slave_account) return false;

    // Check for critical errors related to accounts
    const accountErrors = validation.errors.filter(err =>
      err === validationMessages.sameAccountError ||
      err.includes(validationMessages.duplicateSettings.split('{id}')[0]) || // Approximate check for duplicate
      err.includes(validationMessages.circularReference.split('{slave}')[0]) // Approximate check for circular
    );

    return accountErrors.length === 0;
  }, [formData.master_account, formData.slave_account, validation.errors, validationMessages]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>
            {content.createTitle.value} {step > 1 && `(${step}/2)`}
          </DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="flex flex-col overflow-hidden">
          <div className="space-y-4 overflow-y-auto pr-2 min-h-[300px]">

            {step === 1 && (
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

            {step === 2 && (
              /* Copy Settings Section */
              <div className="space-y-4">
                <div className="space-y-1">
                  <h3 className="text-sm font-medium flex items-center gap-2">
                    <span className="text-lg">⚙️</span>
                    {content.copySettingsLabel?.value || "Copy Settings"}
                  </h3>
                  <p className="text-xs text-muted-foreground">
                    Configure how trades are copied.
                  </p>
                </div>

                {/* Lot Multiplier */}
                <div>
                  <Label htmlFor="lot_multiplier">
                    {content.lotMultiplier?.value || "Lot Multiplier"}
                  </Label>
                  <Input
                    id="lot_multiplier"
                    type="number"
                    step="0.01"
                    min="0.01"
                    max="100"
                    value={formData.lot_multiplier}
                    onChange={(e) => setFormData({ ...formData, lot_multiplier: parseFloat(e.target.value) || 0 })}
                    required
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    {content.lotMultiplierDescription?.value || "Multiplier for lot size (e.g. 1.0 = same size, 0.5 = half size)"}
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
                    {content.reverseTrade?.value || "Reverse Trade"} - {content.reverseDescription?.value || "Copy trades in opposite direction"}
                  </Label>
                </div>

                {/* Symbol Filters Section */}
                <div className="space-y-1 mt-6">
                  <h3 className="text-sm font-medium flex items-center gap-2">
                    <span className="text-lg">🔍</span>
                    Symbol Filters (Optional)
                  </h3>
                  <p className="text-xs text-muted-foreground">
                    Configure symbol name transformations for this connection.
                  </p>
                </div>

                {/* Symbol Prefix */}
                <div>
                  <Label htmlFor="symbol_prefix">
                    Symbol Prefix
                  </Label>
                  <Input
                    id="symbol_prefix"
                    type="text"
                    placeholder="e.g. 'pro.' or 'FX.'"
                    value={formData.symbol_prefix}
                    onChange={(e) => setFormData({ ...formData, symbol_prefix: e.target.value })}
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    Prefix to add to symbol names (e.g., EURUSD → pro.EURUSD)
                  </p>
                </div>

                {/* Symbol Suffix */}
                <div>
                  <Label htmlFor="symbol_suffix">
                    Symbol Suffix
                  </Label>
                  <Input
                    id="symbol_suffix"
                    type="text"
                    placeholder="e.g. '.m' or '-ECN'"
                    value={formData.symbol_suffix}
                    onChange={(e) => setFormData({ ...formData, symbol_suffix: e.target.value })}
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    Suffix to add to symbol names (e.g., EURUSD → EURUSD.m)
                  </p>
                </div>

                {/* Symbol Mappings */}
                <div>
                  <Label htmlFor="symbol_mappings">
                    Symbol Mappings
                  </Label>
                  <Input
                    id="symbol_mappings"
                    type="text"
                    placeholder="e.g. 'XAUUSD=GOLD,EURUSD=EUR'"
                    value={formData.symbol_mappings}
                    onChange={(e) => setFormData({ ...formData, symbol_mappings: e.target.value })}
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    Map source symbols to target symbols (comma-separated, format: SOURCE=TARGET)
                  </p>
                </div>
              </div>
            )}

            {/* Validation Messages - Show always if present, but contextually relevant */}
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
            <div className="flex gap-2 justify-between w-full">
              <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
                {content.cancel.value}
              </Button>

              <div className="flex gap-2">
                {step === 2 && (
                  <Button type="button" variant="outline" onClick={() => setStep(1)}>
                    Back
                  </Button>
                )}

                {step === 1 ? (
                  <Button
                    type="button"
                    onClick={(e) => {
                      e.preventDefault();
                      setStep(2);
                    }}
                    disabled={!isStep1Valid}
                  >
                    Next
                  </Button>
                ) : (
                  <Button type="submit" disabled={!validation.isValid}>
                    {content.saveAndEnable.value}
                  </Button>
                )}
              </div>
            </div>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
