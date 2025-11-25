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
import type { LotCalculationMode } from '@/types';

export interface SlaveSettingsFormData {
  lot_calculation_mode: LotCalculationMode;
  lot_multiplier: number;
  reverse_trade: boolean;
  symbol_prefix: string;
  symbol_suffix: string;
  symbol_mappings: string;
  source_lot_min: number | null;
  source_lot_max: number | null;
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

      {/* Symbol Filters Section */}
      <DrawerSection bordered>
        <DrawerSectionHeader
          title={content.symbolFiltersTitle?.value || "Symbol Filters"}
          description={content.symbolFiltersDescription?.value || "Configure symbol name transformations for this connection."}
        />
        <DrawerSectionContent>
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
        </DrawerSectionContent>
      </DrawerSection>
    </div>
  );
}
