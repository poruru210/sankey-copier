'use client';

// SlaveSettingsForm - Shared form component for slave copy settings
// Used by both CreateConnectionDialog (Step 2) and EditConnectionDrawer
// Contains: lot_multiplier, reverse_trade, symbol filters (prefix, suffix, mappings)

import { useIntlayer } from 'next-intlayer';
import { Input } from '@/components/ui/input';
import { Checkbox } from '@/components/ui/checkbox';
import {
  DrawerSection,
  DrawerSectionHeader,
  DrawerSectionContent,
  DrawerFormField,
} from '@/components/ui/drawer-section';
import { SymbolMappingInput } from '@/components/SymbolMappingInput';

export interface SlaveSettingsFormData {
  lot_multiplier: number;
  reverse_trade: boolean;
  symbol_prefix: string;
  symbol_suffix: string;
  symbol_mappings: string;
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
          {/* Lot Multiplier */}
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
