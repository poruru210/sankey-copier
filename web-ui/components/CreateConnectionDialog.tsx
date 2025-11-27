'use client';

import { useState, useEffect, useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { useAtomValue } from 'jotai';
import { Drawer, DrawerContent, DrawerHeader, DrawerTitle } from '@/components/ui/drawer';
import { useMediaQuery } from '@/hooks/useMediaQuery';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { SimpleAccountSelector } from '@/components/SimpleAccountSelector';
import { useSettingsValidation } from '@/hooks/useSettingsValidation';
import type { CreateSettingsRequest, EaConnection, CopySettings, TradeGroup, TradeGroupMember, LotCalculationMode, SyncMode } from '@/types';
import { Input } from '@/components/ui/input';
import { AlertCircle, AlertTriangle } from 'lucide-react';
import { apiClientAtom } from '@/lib/atoms/site';
import { SlaveSettingsForm, type SlaveSettingsFormData } from '@/components/SlaveSettingsForm';
import { DRAWER_SIZE_SETTINGS } from '@/lib/ui-constants';
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
  DrawerSection,
  DrawerSectionHeader,
  DrawerSectionContent,
  DrawerFormField,
} from '@/components/ui/drawer-section';

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

  // Responsive: right drawer for desktop, bottom drawer for mobile
  const isDesktop = useMediaQuery('(min-width: 768px)');
  const side = isDesktop ? 'right' : 'bottom';

  // Reset stepper when dialog opens/closes
  const [key, setKey] = useState(0);
  useEffect(() => {
    if (open) {
      setKey(prev => prev + 1);
    }
  }, [open]);

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
          <DrawerTitle>{content.createTitle.value}</DrawerTitle>
        </DrawerHeader>
        <Stepper key={key} steps={STEPS} allowStepNavigation initialStep={0}>
          <CreateConnectionForm
            onClose={() => onOpenChange(false)}
            onCreate={onCreate}
            connections={connections}
            existingSettings={existingSettings}
          />
        </Stepper>
      </DrawerContent>
    </Drawer>
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

  const [formData, setFormData] = useState<{
    master_account: string;
    slave_account: string;
    lot_calculation_mode: LotCalculationMode;
    lot_multiplier: number;
    reverse_trade: boolean;
    symbol_prefix: string;
    symbol_suffix: string;
    symbol_mappings: string;
    source_lot_min: number | null;
    source_lot_max: number | null;
    // Open Sync Policy
    sync_mode: SyncMode;
    limit_order_expiry_min: number | null;
    market_sync_max_pips: number | null;
    max_slippage: number | null;
    copy_pending_orders: boolean;
    // Trade Execution
    max_retries: number;
    max_signal_delay_ms: number;
    use_pending_order_for_delayed: boolean;
    // Filter settings
    allowed_magic_numbers: string;
  }>({
    master_account: '',
    slave_account: '',
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


  // Initialize form data on mount
  useEffect(() => {
    setFormData({
      master_account: '',
      slave_account: '',
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
    // Skip update if value hasn't changed OR if trying to set empty string when we have a value
    if (formData.master_account === value || (!value && formData.master_account)) {
      return;
    }

    setFormData({ ...formData, master_account: value });
  };

  const handleSlaveChange = (value: string) => {
    // Skip update if value hasn't changed OR if trying to set empty string when we have a value
    if (formData.slave_account === value || (!value && formData.slave_account)) {
      return;
    }
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
        lot_calculation_mode: formData.lot_calculation_mode,
        lot_multiplier: formData.lot_multiplier,
        reverse_trade: formData.reverse_trade,
        status: 0, // STATUS_DISABLED (default OFF)
        symbol_prefix: formData.symbol_prefix || undefined,
        symbol_suffix: formData.symbol_suffix || undefined,
        symbol_mappings: formData.symbol_mappings || undefined,
        source_lot_min: formData.source_lot_min,
        source_lot_max: formData.source_lot_max,
        // Open Sync Policy
        sync_mode: formData.sync_mode,
        limit_order_expiry_min: formData.limit_order_expiry_min,
        market_sync_max_pips: formData.market_sync_max_pips,
        max_slippage: formData.max_slippage,
        copy_pending_orders: formData.copy_pending_orders,
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
            <div className="space-y-6">
              {/* Master Account Selection */}
              <DrawerSection>
                <DrawerSectionHeader
                  title={content.masterAccountLabel.value}
                  description={content.masterAccountDescription.value}
                />
                <SimpleAccountSelector
                  label=""
                  value={formData.master_account}
                  onChange={handleMasterChange}
                  connections={connections}
                  filterType="Master"
                  required
                />
              </DrawerSection>

              {/* Slave Account Selection */}
              <DrawerSection bordered>
                <DrawerSectionHeader
                  title={content.slaveAccountLabel.value}
                  description={content.slaveAccountDescription.value}
                />
                <SimpleAccountSelector
                  label=""
                  value={formData.slave_account}
                  onChange={handleSlaveChange}
                  connections={connections}
                  filterType="Slave"
                  required
                />
              </DrawerSection>
            </div>
          </Step>

          {/* Step 1: Master Settings */}
          <Step stepIndex={1}>
            <div className="space-y-6">
              <DrawerSection>
                <DrawerSectionHeader
                  title="Master Settings (Global)"
                  description="These settings apply to all slaves connected to this master."
                />

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
              </DrawerSection>

              {/* Symbol Rules Section */}
              <DrawerSection bordered>
                <DrawerSectionHeader
                  title="Symbol Rules"
                  description="Configure symbol name transformations that the Master EA will apply before broadcasting."
                />
                <DrawerSectionContent>
                  {/* Symbol Prefix */}
                  <DrawerFormField
                    label="Symbol Prefix"
                    description="Master will remove this prefix when broadcasting symbols (e.g., pro.EURUSD → EURUSD)"
                    htmlFor="master_symbol_prefix"
                  >
                    <Input
                      id="master_symbol_prefix"
                      type="text"
                      placeholder="e.g. 'pro.' or 'FX.'"
                      value={masterSettings.symbol_prefix}
                      onChange={(e) => setMasterSettings({ ...masterSettings, symbol_prefix: e.target.value })}
                      disabled={loadingMembers}
                    />
                  </DrawerFormField>

                  {/* Symbol Suffix */}
                  <DrawerFormField
                    label="Symbol Suffix"
                    description="Master will remove this suffix when broadcasting symbols (e.g., EURUSD.m → EURUSD)"
                    htmlFor="master_symbol_suffix"
                  >
                    <Input
                      id="master_symbol_suffix"
                      type="text"
                      placeholder="e.g. '.m' or '-ECN'"
                      value={masterSettings.symbol_suffix}
                      onChange={(e) => setMasterSettings({ ...masterSettings, symbol_suffix: e.target.value })}
                      disabled={loadingMembers}
                    />
                  </DrawerFormField>
                </DrawerSectionContent>
              </DrawerSection>
            </div>
          </Step>

          {/* Step 2: Slave Settings (using shared component) */}
          <Step stepIndex={2}>
            <SlaveSettingsForm
              formData={{
                lot_calculation_mode: formData.lot_calculation_mode,
                lot_multiplier: formData.lot_multiplier,
                reverse_trade: formData.reverse_trade,
                symbol_prefix: formData.symbol_prefix,
                symbol_suffix: formData.symbol_suffix,
                symbol_mappings: formData.symbol_mappings,
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
                allowed_magic_numbers: formData.allowed_magic_numbers,
              }}
              onChange={(data) => setFormData({ ...formData, ...data })}
            />
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
