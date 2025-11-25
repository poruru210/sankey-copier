'use client';

// SymbolMappingInput - Pair-based input component for symbol mappings
// Allows users to add/remove symbol mapping pairs (SOURCE → TARGET)
// Converts to/from comma-separated string format: "XAUUSD=GOLD,EURUSD=EUR"

import { useState, useEffect, useCallback } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Plus, X } from 'lucide-react';
import { Caption } from '@/components/ui/typography';

interface SymbolMapping {
  id: string;
  source: string;
  target: string;
}

interface SymbolMappingInputProps {
  /** Current value as comma-separated string (e.g., "XAUUSD=GOLD,EURUSD=EUR") */
  value: string;
  /** Callback when value changes */
  onChange: (value: string) => void;
  /** Disable all inputs */
  disabled?: boolean;
}

/**
 * Parse comma-separated mapping string to array of mapping objects
 */
function parseMappings(value: string): SymbolMapping[] {
  if (!value || value.trim() === '') {
    return [];
  }

  return value
    .split(',')
    .map((pair, index) => {
      const [source, target] = pair.split('=').map(s => s.trim());
      return {
        id: `mapping-${index}-${Date.now()}`,
        source: source || '',
        target: target || '',
      };
    })
    .filter(m => m.source || m.target);
}

/**
 * Serialize array of mapping objects to comma-separated string
 */
function serializeMappings(mappings: SymbolMapping[]): string {
  return mappings
    .filter(m => m.source && m.target)
    .map(m => `${m.source}=${m.target}`)
    .join(',');
}

export function SymbolMappingInput({
  value,
  onChange,
  disabled = false,
}: SymbolMappingInputProps) {
  const content = useIntlayer('settings-dialog');
  const [mappings, setMappings] = useState<SymbolMapping[]>([]);

  // Parse incoming value on mount or when value changes externally
  useEffect(() => {
    const parsed = parseMappings(value);
    setMappings(parsed);
  }, [value]);

  // Notify parent of changes
  const notifyChange = useCallback((newMappings: SymbolMapping[]) => {
    const serialized = serializeMappings(newMappings);
    onChange(serialized);
  }, [onChange]);

  const handleAddMapping = () => {
    const newMapping: SymbolMapping = {
      id: `mapping-${Date.now()}`,
      source: '',
      target: '',
    };
    const newMappings = [...mappings, newMapping];
    setMappings(newMappings);
    // Don't notify yet - empty mappings are filtered out
  };

  const handleRemoveMapping = (id: string) => {
    const newMappings = mappings.filter(m => m.id !== id);
    setMappings(newMappings);
    notifyChange(newMappings);
  };

  const handleMappingChange = (id: string, field: 'source' | 'target', fieldValue: string) => {
    const newMappings = mappings.map(m =>
      m.id === id ? { ...m, [field]: fieldValue.toUpperCase() } : m
    );
    setMappings(newMappings);
    notifyChange(newMappings);
  };

  return (
    <div className="space-y-2">
      {/* Existing mappings */}
      {mappings.length > 0 && (
        <div className="space-y-2">
          {mappings.map((mapping) => (
            <div key={mapping.id} className="flex items-center gap-2">
              <Input
                type="text"
                placeholder={content.sourceSymbolPlaceholder?.value || 'e.g. XAUUSD'}
                value={mapping.source}
                onChange={(e) => handleMappingChange(mapping.id, 'source', e.target.value)}
                disabled={disabled}
                className="flex-1"
              />
              <span className="text-muted-foreground text-sm shrink-0">→</span>
              <Input
                type="text"
                placeholder={content.targetSymbolPlaceholder?.value || 'e.g. GOLD'}
                value={mapping.target}
                onChange={(e) => handleMappingChange(mapping.id, 'target', e.target.value)}
                disabled={disabled}
                className="flex-1"
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                onClick={() => handleRemoveMapping(mapping.id)}
                disabled={disabled}
                className="shrink-0 h-9 w-9 text-muted-foreground hover:text-destructive"
                aria-label={content.removeMapping?.value || 'Remove'}
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          ))}
        </div>
      )}

      {/* Add mapping button */}
      <Button
        type="button"
        variant="outline"
        size="sm"
        onClick={handleAddMapping}
        disabled={disabled}
        className="w-full"
      >
        <Plus className="h-4 w-4 mr-2" />
        {content.addMapping?.value || 'Add Mapping'}
      </Button>

      {/* Helper text when no mappings */}
      {mappings.length === 0 && (
        <Caption className="text-center">
          {content.symbolMappingsDescription?.value || 'Map source symbols to target symbols for this connection.'}
        </Caption>
      )}
    </div>
  );
}
