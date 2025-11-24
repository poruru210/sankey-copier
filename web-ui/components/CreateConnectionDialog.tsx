'use client';

import { useState, useEffect, useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { useAtomValue } from 'jotai';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { SimpleAccountSelector } from '@/components/SimpleAccountSelector';
import { useSettingsValidation } from '@/hooks/useSettingsValidation';
import type { CreateSettingsRequest, EaConnection, CopySettings, TradeGroup, TradeGroupMember } from '@/types';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import { AlertCircle, AlertTriangle } from 'lucide-react';
import { apiClientAtom } from '@/lib/atoms/site';
import {
  Stepper,
  StepperHeader,
  StepperContent,
  Step,
  StepperActions,
  useStepper,
  type Step as StepType,
} from '@/components/ui/stepper';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';

interface CreateConnectionDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreate: (data: CreateSettingsRequest) => void;
  connections: EaConnection[];
  existingSettings: CopySettings[];
}

// Define steps for the Stepper
const STEPS: StepType[] = [
  {
    id: 'accounts',
    label: 'Accounts',
    description: 'Select Master & Slave',
  },
  {
    id: 'master-settings',
    label: 'Master Settings',
    description: 'Global configuration',
  },
  {
    id: 'slave-settings',
    label: 'Slave Settings',
    description: 'Copy configuration',
  },
];

export function CreateConnectionDialog({
  open,
  onOpenChange,
  onCreate,
  connections,
  existingSettings
}: CreateConnectionDialogProps) {
  const content = useIntlayer('settings-dialog');

  // Reset stepper when dialog opens/closes
  const [key, setKey] = useState(0);
  useEffect(() => {
    if (open) {
      setKey(prev => prev + 1);
    }
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="!max-w-7xl !max-h-[90vh] flex flex-col overflow-hidden">
        <DialogHeader>
          <DialogTitle>{content.createTitle.value}</DialogTitle>
        </DialogHeader>
        <Stepper key={key} steps={STEPS} allowStepNavigation initialStep={0}>
          <CreateConnectionForm
            onClose={() => onOpenChange(false)}
            onCreate={onCreate}
            connections={connections}
            existingSettings={existingSettings}
          />
        </Stepper>
      </DialogContent>
    </Dialog>
  );
}

// Separate form component to access Stepper context
interface CreateConnectionFormProps {
  onClose: () => void;
  onCreate: (data: CreateSettingsRequest) => void;
  connections: EaConnection[];
  existingSettings: CopySettings[];
}

function CreateConnectionForm({
  onClose,
  onCreate,
  connections,
  existingSettings,
}: CreateConnectionFormProps) {
  const content = useIntlayer('settings-dialog');
  const apiClient = useAtomValue(apiClientAtom);
  const { currentStep, setCurrentStep, setStepComplete } = useStepper();

  // Track master settings and existing members
  const [masterSettings, setMasterSettings] = useState({
    symbol_prefix: '',
    symbol_suffix: '',
  });
  const [existingMembers, setExistingMembers] = useState<TradeGroupMember[]>([]);
  const [existingTradeGroup, setExistingTradeGroup] = useState<TradeGroup | null>(null);
  const [loadingMembers, setLoadingMembers] = useState(false);

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

  // Track accordion expansion state for Step 1 (Master) and Step 2 (Slave)
  const [openMasterAccordion, setOpenMasterAccordion] = useState<string>('master-symbols');
  const [openSlaveAccordion, setOpenSlaveAccordion] = useState<string>('basic');

  // Initialize form data on mount
  useEffect(() => {
    setFormData({
      master_account: '',
      slave_account: '',
      lot_multiplier: 1.0,
      reverse_trade: false,
      symbol_prefix: '',
      symbol_suffix: '',
      symbol_mappings: '',
    });
    setMasterSettings({
      symbol_prefix: '',
      symbol_suffix: '',
    });
    setExistingMembers([]);
    setExistingTradeGroup(null);
  }, []);

  // Fetch existing TradeGroup and members when master account is selected
  useEffect(() => {
    const fetchMasterData = async () => {
      if (!apiClient || !formData.master_account) {
        setExistingMembers([]);
        setExistingTradeGroup(null);
        return;
      }

      setLoadingMembers(true);
      try {
        // Fetch TradeGroup (Master settings)
        const tradeGroup = await apiClient.getTradeGroup(formData.master_account);
        setExistingTradeGroup(tradeGroup);

        // Pre-fill master settings from existing TradeGroup
        setMasterSettings({
          symbol_prefix: tradeGroup.master_settings.symbol_prefix || '',
          symbol_suffix: tradeGroup.master_settings.symbol_suffix || '',
        });

        // Fetch existing members
        const members = await apiClient.listTradeGroupMembers(formData.master_account);
        setExistingMembers(members || []);
      } catch (err) {
        // If TradeGroup doesn't exist yet, it's OK (will be created on first member)
        console.log('No existing TradeGroup for', formData.master_account);
        setExistingMembers([]);
        setExistingTradeGroup(null);
        setMasterSettings({
          symbol_prefix: '',
          symbol_suffix: '',
        });
      } finally {
        setLoadingMembers(false);
      }
    };

    fetchMasterData();
  }, [apiClient, formData.master_account]);

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

  const handleNext = async () => {
    // Step 0: Account Selection → Go to Step 1 (Master Settings)
    if (currentStep === 0) {
      if (isStep1Valid) {
        setStepComplete(0, true);
        setCurrentStep(1);
      }
      return;
    }

    // Step 1: Master Settings → Save and go to Step 2 (Slave Settings)
    if (currentStep === 1) {
      try {
        // Only update master settings if they exist or have values
        if (apiClient && formData.master_account && (masterSettings.symbol_prefix || masterSettings.symbol_suffix)) {
          await apiClient.updateTradeGroupSettings(formData.master_account, {
            symbol_prefix: masterSettings.symbol_prefix || null,
            symbol_suffix: masterSettings.symbol_suffix || null,
            config_version: existingTradeGroup?.master_settings.config_version || 0,
          });
        }
        setStepComplete(1, true);
        setCurrentStep(2);
      } catch (err) {
        console.error('Failed to update master settings:', err);
        // Still proceed to step 2 even if master settings update fails
        setStepComplete(1, true);
        setCurrentStep(2);
      }
      return;
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Step 2: Slave Settings → Create member
    if (currentStep === 2) {
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
      setStepComplete(2, true);
      onClose();
    }
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
    <>
      <StepperHeader className="mb-6" />

      <form onSubmit={handleSubmit} className="flex flex-col flex-1 overflow-hidden">
        <StepperContent className="overflow-y-auto pr-2">
          {/* Step 0: Account Selection */}
          <Step stepIndex={0}>
            <div className="space-y-4">
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
            </div>
          </Step>

          {/* Step 1: Master Settings */}
          <Step stepIndex={1}>
            <div className="space-y-4">
              <div className="space-y-1">
                <h3 className="text-sm font-medium flex items-center gap-2">
                  <span className="text-lg">📊</span>
                  Master Settings (Global)
                </h3>
                <p className="text-xs text-muted-foreground">
                  These settings apply to all slaves connected to this master.
                </p>
              </div>

              {/* Warning if other slaves exist */}
              {existingMembers.length > 0 && (
                <div className="rounded-md bg-yellow-50 dark:bg-yellow-950 p-3 border border-yellow-200 dark:border-yellow-800">
                  <div className="flex">
                    <AlertTriangle className="h-4 w-4 text-yellow-400 mr-2 flex-shrink-0 mt-0.5" />
                    <div className="flex-1">
                      <h3 className="text-xs font-medium text-yellow-800 dark:text-yellow-200">
                        Existing Connections
                      </h3>
                      <p className="mt-1 text-xs text-yellow-700 dark:text-yellow-300">
                        This master has {existingMembers.length} existing slave{existingMembers.length > 1 ? 's' : ''}.
                        Changing these settings will affect all slaves connected to this master.
                      </p>
                    </div>
                  </div>
                </div>
              )}

              {/* Accordion Layout for Master Settings */}
              <Accordion
                type="single"
                value={openMasterAccordion}
                onValueChange={setOpenMasterAccordion}
                collapsible
                className="w-full"
              >
                {/* Master Symbol Filters */}
                <AccordionItem value="master-symbols">
                  <AccordionTrigger>
                    <div className="flex items-center gap-2">
                      <span className="text-lg">🔍</span>
                      <span>Symbol Filters (Optional)</span>
                    </div>
                  </AccordionTrigger>
                  <AccordionContent>
                    <div className="space-y-4 pt-2">
                      <p className="text-xs text-muted-foreground">
                        Configure symbol name transformations that the Master EA will apply before broadcasting.
                      </p>

                      {/* Symbol Prefix */}
                      <div>
                        <Label htmlFor="master_symbol_prefix">
                          Symbol Prefix
                        </Label>
                        <Input
                          id="master_symbol_prefix"
                          type="text"
                          placeholder="e.g. 'pro.' or 'FX.'"
                          value={masterSettings.symbol_prefix}
                          onChange={(e) => setMasterSettings({ ...masterSettings, symbol_prefix: e.target.value })}
                          disabled={loadingMembers}
                        />
                        <p className="text-xs text-muted-foreground mt-1">
                          Master will remove this prefix when broadcasting symbols (e.g., pro.EURUSD → EURUSD)
                        </p>
                      </div>

                      {/* Symbol Suffix */}
                      <div>
                        <Label htmlFor="master_symbol_suffix">
                          Symbol Suffix
                        </Label>
                        <Input
                          id="master_symbol_suffix"
                          type="text"
                          placeholder="e.g. '.m' or '-ECN'"
                          value={masterSettings.symbol_suffix}
                          onChange={(e) => setMasterSettings({ ...masterSettings, symbol_suffix: e.target.value })}
                          disabled={loadingMembers}
                        />
                        <p className="text-xs text-muted-foreground mt-1">
                          Master will remove this suffix when broadcasting symbols (e.g., EURUSD.m → EURUSD)
                        </p>
                      </div>
                    </div>
                  </AccordionContent>
                </AccordionItem>
              </Accordion>
            </div>
          </Step>

          {/* Step 2: Slave Settings */}
          <Step stepIndex={2}>
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

              {/* Accordion Layout */}
              <Accordion
                type="single"
                value={openSlaveAccordion}
                onValueChange={setOpenSlaveAccordion}
                collapsible
                className="w-full"
              >
                {/* Basic Copy Settings */}
                <AccordionItem value="basic">
                  <AccordionTrigger>
                    <div className="flex items-center gap-2">
                      <span className="text-lg">📊</span>
                      <span>Basic Copy Settings</span>
                    </div>
                  </AccordionTrigger>
                  <AccordionContent>
                    <div className="space-y-4 pt-2">
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
                    </div>
                  </AccordionContent>
                </AccordionItem>

                {/* Symbol Filters */}
                <AccordionItem value="filters">
                  <AccordionTrigger>
                    <div className="flex items-center gap-2">
                      <span className="text-lg">🔍</span>
                      <span>Symbol Filters (Optional)</span>
                    </div>
                  </AccordionTrigger>
                  <AccordionContent>
                    <div className="space-y-4 pt-2">
                      <p className="text-xs text-muted-foreground">
                        Configure symbol name transformations for this connection.
                      </p>

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
                  </AccordionContent>
                </AccordionItem>
              </Accordion>
            </div>
          </Step>

          {/* Validation Messages - Show always if present, but contextually relevant */}
          {validation.errors.length > 0 && (
            <div className="rounded-md bg-red-50 dark:bg-red-950 p-3 border border-red-200 dark:border-red-800 mt-4">
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
            <div className="rounded-md bg-yellow-50 dark:bg-yellow-950 p-3 border border-yellow-200 dark:border-yellow-800 mt-4">
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
        </StepperContent>

        <StepperActions className="flex-shrink-0 pt-4 border-t">
          <Button type="button" variant="ghost" onClick={onClose}>
            {content.cancel.value}
          </Button>

          <div className="flex gap-2">
            {/* Back button for steps 1 and 2 */}
            {currentStep > 0 && (
              <Button
                type="button"
                variant="outline"
                onClick={() => setCurrentStep(currentStep - 1)}
              >
                Back
              </Button>
            )}

            {/* Next button for steps 0 and 1 */}
            {currentStep < 2 && (
              <Button
                type="button"
                onClick={handleNext}
                disabled={currentStep === 0 && !isStep1Valid}
              >
                {currentStep === 1 && loadingMembers ? 'Loading...' : 'Next'}
              </Button>
            )}

            {/* Submit button for step 2 */}
            {currentStep === 2 && (
              <Button type="submit" disabled={!validation.isValid}>
                {content.saveAndEnable.value}
              </Button>
            )}
          </div>
        </StepperActions>
      </form>
    </>
  );
}
