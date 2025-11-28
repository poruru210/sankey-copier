'use client';

// SlaveSettingsForm - Shared form component for slave copy settings
// Used by both CreateConnectionDialog (Step 2) and EditConnectionDrawer
// Contains: lot_calculation_mode, lot_multiplier, source_lot_min/max, reverse_trade, symbol filters

import { useIntlayer } from 'next-intlayer';
import { Input } from '@/components/ui/input';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import {
  DrawerSection,
  DrawerSectionHeader,
  DrawerSectionContent,
  DrawerFormField,
} from '@/components/ui/drawer-section';
import { SymbolMappingInput } from '@/components/SymbolMappingInput';
import type { LotCalculationMode, SyncMode } from '@/types';

export interface SlaveSettingsFormData {
  lot_calculation_mode: LotCalculationMode;
  lot_multiplier: number;
  reverse_trade: boolean;
  symbol_prefix: string;
  symbol_suffix: string;
  symbol_mappings: string;
  source_lot_min: number | null;
  source_lot_max: number | null;
  // Open Sync Policy settings
  sync_mode: SyncMode;
  limit_order_expiry_min: number | null;
  market_sync_max_pips: number | null;
  max_slippage: number | null;
  copy_pending_orders: boolean;
  // Trade Execution settings
  max_retries: number;
  max_signal_delay_ms: number;
  use_pending_order_for_delayed: boolean;
  // Filter settings
  allowed_magic_numbers: string; // Comma-separated list
}

interface SlaveSettingsFormProps {
  formData: SlaveSettingsFormData;
  onChange: (data: SlaveSettingsFormData) => void;
  disabled?: boolean;
}

export function SlaveSettingsForm({
  formData,
  onChange,
  disabled = false,
}: SlaveSettingsFormProps) {
  const content = useIntlayer('settings-dialog');

  const handleChange = <K extends keyof SlaveSettingsFormData>(
    key: K,
    value: SlaveSettingsFormData[K]
  ) => {
    onChange({ ...formData, [key]: value });
  };

  return (
    <div className="space-y-6">
      {/* Copy Settings Section */}
      <DrawerSection>
        <DrawerSectionHeader
          title={content.copySettingsLabel?.value || "Copy Settings"}
          description={content.copySettingsDescription?.value || "Configure how trades are copied."}
        />
        <DrawerSectionContent>
          {/* Lot Calculation Mode */}
          <DrawerFormField
            label={content.lotCalculationMode?.value || "Lot Calculation Mode"}
            description={content.lotCalculationModeDescription?.value || "How to calculate lot size for copied trades"}
            htmlFor="lot_calculation_mode"
          >
            <RadioGroup
              value={formData.lot_calculation_mode}
              onValueChange={(value) => handleChange('lot_calculation_mode', value as LotCalculationMode)}
              disabled={disabled}
              className="flex flex-col space-y-2"
            >
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="multiplier" id="mode_multiplier" />
                <Label htmlFor="mode_multiplier" className="text-sm font-normal cursor-pointer">
                  {content.lotModeMultiplier?.value || "Fixed Multiplier"} - {content.lotModeMultiplierDesc?.value || "Use fixed multiplier value"}
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="margin_ratio" id="mode_margin_ratio" />
                <Label htmlFor="mode_margin_ratio" className="text-sm font-normal cursor-pointer">
                  {content.lotModeMarginRatio?.value || "Margin Ratio"} - {content.lotModeMarginRatioDesc?.value || "Calculate based on equity ratio (slave/master)"}
                </Label>
              </div>
            </RadioGroup>
          </DrawerFormField>

          {/* Lot Multiplier - only show when mode is multiplier */}
          {formData.lot_calculation_mode === 'multiplier' && (
            <DrawerFormField
              label={content.lotMultiplier?.value || "Lot Multiplier"}
              description={content.lotMultiplierDescription?.value || "Multiplier for lot size (e.g. 1.0 = same size, 0.5 = half size)"}
              htmlFor="lot_multiplier"
            >
              <Input
                id="lot_multiplier"
                type="number"
                step="0.01"
                min="0.01"
                max="100"
                value={formData.lot_multiplier}
                onChange={(e) => handleChange('lot_multiplier', parseFloat(e.target.value) || 0)}
                required
                disabled={disabled}
              />
            </DrawerFormField>
          )}

          {/* Reverse Trade */}
          <div className="flex items-center space-x-2">
            <Checkbox
              id="reverse_trade"
              checked={formData.reverse_trade}
              onCheckedChange={(checked) => handleChange('reverse_trade', checked as boolean)}
              disabled={disabled}
            />
            <label htmlFor="reverse_trade" className="text-sm cursor-pointer">
              {content.reverseTrade?.value || "Reverse Trade"} - {content.reverseDescription?.value || "Copy trades in opposite direction"}
            </label>
          </div>
        </DrawerSectionContent>
      </DrawerSection>

      {/* Lot Filter Section */}
      <DrawerSection bordered>
        <DrawerSectionHeader
          title={content.lotFilterTitle?.value || "Lot Filter"}
          description={content.lotFilterDescription?.value || "Filter trades by source lot size. Leave empty for no filtering."}
        />
        <DrawerSectionContent>
          {/* Source Lot Min */}
          <DrawerFormField
            label={content.sourceLotMin?.value || "Minimum Lot"}
            description={content.sourceLotMinDescription?.value || "Skip trades with lot size smaller than this value"}
            htmlFor="source_lot_min"
          >
            <Input
              id="source_lot_min"
              type="number"
              step="0.01"
              min="0"
              placeholder={content.sourceLotMinPlaceholder?.value || "e.g. 0.01"}
              value={formData.source_lot_min ?? ''}
              onChange={(e) => {
                const val = e.target.value;
                handleChange('source_lot_min', val === '' ? null : parseFloat(val));
              }}
              disabled={disabled}
            />
          </DrawerFormField>

          {/* Source Lot Max */}
          <DrawerFormField
            label={content.sourceLotMax?.value || "Maximum Lot"}
            description={content.sourceLotMaxDescription?.value || "Skip trades with lot size larger than this value"}
            htmlFor="source_lot_max"
          >
            <Input
              id="source_lot_max"
              type="number"
              step="0.01"
              min="0"
              placeholder={content.sourceLotMaxPlaceholder?.value || "e.g. 10.0"}
              value={formData.source_lot_max ?? ''}
              onChange={(e) => {
                const val = e.target.value;
                handleChange('source_lot_max', val === '' ? null : parseFloat(val));
              }}
              disabled={disabled}
            />
          </DrawerFormField>
        </DrawerSectionContent>
      </DrawerSection>

      {/* Symbol Rules Section */}
      <DrawerSection bordered>
        <DrawerSectionHeader
          title={content.symbolFiltersTitle?.value || "Symbol Rules"}
          description={content.symbolFiltersDescription?.value || "Configure symbol name transformations for this connection."}
        />
        <DrawerSectionContent>
          {/* Symbol Mappings */}
          <DrawerFormField
            label={content.symbolMappings?.value || "Symbol Mappings"}
            htmlFor="symbol_mappings"
          >
            <SymbolMappingInput
              value={formData.symbol_mappings}
              onChange={(value) => handleChange('symbol_mappings', value)}
              disabled={disabled}
            />
          </DrawerFormField>

          {/* Symbol Prefix */}
          <DrawerFormField
            label={content.symbolPrefix?.value || "Symbol Prefix"}
            description={content.symbolPrefixDescription?.value || "Prefix to add to symbol names (e.g., EURUSD → pro.EURUSD)"}
            htmlFor="symbol_prefix"
          >
            <Input
              id="symbol_prefix"
              type="text"
              placeholder={content.symbolPrefixPlaceholder?.value || "e.g. 'pro.' or 'FX.'"}
              value={formData.symbol_prefix}
              onChange={(e) => handleChange('symbol_prefix', e.target.value)}
              disabled={disabled}
            />
          </DrawerFormField>

          {/* Symbol Suffix */}
          <DrawerFormField
            label={content.symbolSuffix?.value || "Symbol Suffix"}
            description={content.symbolSuffixDescription?.value || "Suffix to add to symbol names (e.g., EURUSD → EURUSD.m)"}
            htmlFor="symbol_suffix"
          >
            <Input
              id="symbol_suffix"
              type="text"
              placeholder={content.symbolSuffixPlaceholder?.value || "e.g. '.m' or '-ECN'"}
              value={formData.symbol_suffix}
              onChange={(e) => handleChange('symbol_suffix', e.target.value)}
              disabled={disabled}
            />
          </DrawerFormField>
        </DrawerSectionContent>
      </DrawerSection>

      {/* Open Sync Policy Section */}
      <DrawerSection bordered>
        <DrawerSectionHeader
          title={content.syncPolicyTitle?.value || "Open Sync Policy"}
          description={content.syncPolicyDescription?.value || "Configure how existing positions are synchronized when slave connects."}
        />
        <DrawerSectionContent>
          {/* Sync Mode */}
          <DrawerFormField
            label={content.syncMode?.value || "Existing Position Sync"}
            description={content.syncModeDescription?.value || "How to handle existing master positions when slave connects"}
            htmlFor="sync_mode"
          >
            <RadioGroup
              value={formData.sync_mode}
              onValueChange={(value) => handleChange('sync_mode', value as SyncMode)}
              disabled={disabled}
              className="flex flex-col space-y-2"
            >
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="skip" id="sync_skip" />
                <Label htmlFor="sync_skip" className="text-sm font-normal cursor-pointer">
                  {content.syncModeSkip?.value || "Don't Sync"} - {content.syncModeSkipDesc?.value || "Only copy new trades, ignore existing positions"}
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="limit_order" id="sync_limit" />
                <Label htmlFor="sync_limit" className="text-sm font-normal cursor-pointer">
                  {content.syncModeLimitOrder?.value || "Limit Order"} - {content.syncModeLimitOrderDesc?.value || "Sync at Master's open price with time limit"}
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="market_order" id="sync_market" />
                <Label htmlFor="sync_market" className="text-sm font-normal cursor-pointer">
                  {content.syncModeMarketOrder?.value || "Market Order"} - {content.syncModeMarketOrderDesc?.value || "Sync immediately if price deviation is within limit"}
                </Label>
              </div>
            </RadioGroup>
          </DrawerFormField>

          {/* Limit Order Expiry - only show when sync_mode is limit_order */}
          {formData.sync_mode === 'limit_order' && (
            <DrawerFormField
              label={content.limitOrderExpiry?.value || "Limit Order Expiry (minutes)"}
              description={content.limitOrderExpiryDescription?.value || "Time limit for limit orders. 0 = Good Till Cancelled (GTC)."}
              htmlFor="limit_order_expiry_min"
            >
              <Input
                id="limit_order_expiry_min"
                type="number"
                step="1"
                min="0"
                placeholder={content.limitOrderExpiryPlaceholder?.value || "e.g. 60 (0 = GTC)"}
                value={formData.limit_order_expiry_min ?? ''}
                onChange={(e) => {
                  const val = e.target.value;
                  handleChange('limit_order_expiry_min', val === '' ? null : parseInt(val, 10));
                }}
                disabled={disabled}
              />
            </DrawerFormField>
          )}

          {/* Market Sync Max Pips - only show when sync_mode is market_order */}
          {formData.sync_mode === 'market_order' && (
            <DrawerFormField
              label={content.marketSyncMaxPips?.value || "Max Price Deviation (pips)"}
              description={content.marketSyncMaxPipsDescription?.value || "Skip sync if current price differs from open price by more than this value."}
              htmlFor="market_sync_max_pips"
            >
              <Input
                id="market_sync_max_pips"
                type="number"
                step="0.1"
                min="0"
                placeholder={content.marketSyncMaxPipsPlaceholder?.value || "e.g. 10.0"}
                value={formData.market_sync_max_pips ?? ''}
                onChange={(e) => {
                  const val = e.target.value;
                  handleChange('market_sync_max_pips', val === '' ? null : parseFloat(val));
                }}
                disabled={disabled}
              />
            </DrawerFormField>
          )}

          {/* Max Slippage */}
          <DrawerFormField
            label={content.maxSlippage?.value || "Max Slippage (points)"}
            description={content.maxSlippageDescription?.value || "Maximum allowed slippage when opening positions. Leave empty for default (30 points)."}
            htmlFor="max_slippage"
          >
            <Input
              id="max_slippage"
              type="number"
              step="1"
              min="0"
              max="1000"
              placeholder={content.maxSlippagePlaceholder?.value || "e.g. 30"}
              value={formData.max_slippage ?? ''}
              onChange={(e) => {
                const val = e.target.value;
                handleChange('max_slippage', val === '' ? null : parseInt(val, 10));
              }}
              disabled={disabled}
            />
          </DrawerFormField>

          {/* Copy Pending Orders */}
          <div className="flex items-center space-x-2">
            <Checkbox
              id="copy_pending_orders"
              checked={formData.copy_pending_orders}
              onCheckedChange={(checked) => handleChange('copy_pending_orders', checked as boolean)}
              disabled={disabled}
            />
            <label htmlFor="copy_pending_orders" className="text-sm cursor-pointer">
              {content.copyPendingOrders?.value || "Copy Pending Orders"} - {content.copyPendingOrdersDesc?.value || "Also copy limit and stop orders"}
            </label>
          </div>
        </DrawerSectionContent>
      </DrawerSection>

      {/* Trade Execution Settings Section */}
      <DrawerSection bordered>
        <DrawerSectionHeader
          title={content.tradeExecutionTitle?.value || "Trade Execution"}
          description={content.tradeExecutionDescription?.value || "Configure signal processing and order execution behavior."}
        />
        <DrawerSectionContent>
          {/* Max Retries */}
          <DrawerFormField
            label={content.maxRetries?.value || "Max Retries"}
            description={content.maxRetriesDescription?.value || "Maximum number of order retry attempts on failure."}
            htmlFor="max_retries"
          >
            <Input
              id="max_retries"
              type="number"
              step="1"
              min="0"
              max="10"
              placeholder="3"
              value={formData.max_retries}
              onChange={(e) => handleChange('max_retries', parseInt(e.target.value, 10) || 3)}
              disabled={disabled}
            />
          </DrawerFormField>

          {/* Max Signal Delay */}
          <DrawerFormField
            label={content.maxSignalDelay?.value || "Max Signal Delay (ms)"}
            description={content.maxSignalDelayDescription?.value || "Maximum allowed signal delay in milliseconds. Signals older than this are skipped or handled based on the setting below."}
            htmlFor="max_signal_delay_ms"
          >
            <Input
              id="max_signal_delay_ms"
              type="number"
              step="100"
              min="0"
              max="60000"
              placeholder="5000"
              value={formData.max_signal_delay_ms}
              onChange={(e) => handleChange('max_signal_delay_ms', parseInt(e.target.value, 10) || 5000)}
              disabled={disabled}
            />
          </DrawerFormField>

          {/* Use Pending Order for Delayed */}
          <div className="flex items-center space-x-2">
            <Checkbox
              id="use_pending_order_for_delayed"
              checked={formData.use_pending_order_for_delayed}
              onCheckedChange={(checked) => handleChange('use_pending_order_for_delayed', checked as boolean)}
              disabled={disabled}
            />
            <label htmlFor="use_pending_order_for_delayed" className="text-sm cursor-pointer">
              {content.usePendingOrderForDelayed?.value || "Use Pending Order for Delayed Signals"} - {content.usePendingOrderForDelayedDesc?.value || "Place limit order at original price instead of skipping"}
            </label>
          </div>
        </DrawerSectionContent>
      </DrawerSection>

      {/* Magic Number Filter Section */}
      <DrawerSection bordered>
        <DrawerSectionHeader
          title={content.magicFilterTitle?.value || "Magic Number Filter"}
          description={content.magicFilterDescription?.value || "Filter which trades to copy based on magic number. Leave empty to copy all trades."}
        />
        <DrawerSectionContent>
          {/* Allowed Magic Numbers */}
          <DrawerFormField
            label={content.allowedMagicNumbers?.value || "Allowed Magic Numbers"}
            description={content.allowedMagicNumbersDescription?.value || "Comma-separated list of magic numbers to copy. Only trades with these magic numbers will be copied."}
            htmlFor="allowed_magic_numbers"
          >
            <Input
              id="allowed_magic_numbers"
              type="text"
              placeholder={content.allowedMagicNumbersPlaceholder?.value || "e.g. 12345, 67890"}
              value={formData.allowed_magic_numbers}
              onChange={(e) => handleChange('allowed_magic_numbers', e.target.value)}
              disabled={disabled}
            />
          </DrawerFormField>
        </DrawerSectionContent>
      </DrawerSection>
    </div>
  );
}
