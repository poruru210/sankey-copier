import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { SymbolContext } from "@/types"
import { SlaveSettingsFormData } from "./SlaveSettingsForm"
import { Wand2 } from "lucide-react"

interface DetectedSettingsAlertProps {
    detectedContext: SymbolContext;
    formData: SlaveSettingsFormData;
    onApply: (changes: Partial<SlaveSettingsFormData>) => void;
}

export function DetectedSettingsAlert({
    detectedContext,
    formData,
    onApply,
}: DetectedSettingsAlertProps) {
    const { detected_prefix, detected_suffix, available_special_symbols } = detectedContext;

    // 1. Prefix/Suffix Check
    const diffPrefix = detected_prefix && detected_prefix !== formData.symbol_prefix;
    const diffSuffix = detected_suffix && detected_suffix !== formData.symbol_suffix;

    // 2. Mapping Check
    // Common synonym dictionary (Master Symbol -> Base Slave Symbol)
    const SYNONYM_DICT: Record<string, string[]> = {
        'XAUUSD': ['GOLD', 'XAUUSD'],
        'BTCUSD': ['BTC', 'BTCUSD'],
        'US30': ['DJ30', 'US30'],
        'NAS100': ['NDX100', 'US100', 'NAS100'],
    };

    const newMappings: string[] = [];

    // Helper to check if a mapping for source already exists
    const existingMappings = (formData.symbol_mappings || '').split(',').map(s => s.trim());
    const hasMappingFor = (source: string) => existingMappings.some(m => m.startsWith(`${source}=`));

    // Check available special symbols against dictionary
    if (available_special_symbols && available_special_symbols.length > 0) {
        for (const [masterBase, slaveCandidates] of Object.entries(SYNONYM_DICT)) {
            // If we already map this master base, skip
            if (hasMappingFor(masterBase)) continue;

            for (const candidateBase of slaveCandidates) {
                // Skip if candidate is same as master (redundant identity mapping)
                if (candidateBase === masterBase) continue;

                // Construct full slave symbol candidates
                // e.g., "GOLD", "GOLD.m", "pro.GOLD"
                // We match strictly what is in available_special_symbols

                // Exact match check (or match with detected prefix/suffix logic applied? No, special symbols usually override standard rules)
                // But often special symbols have suffixes too.
                // Let's simplified check: if available_special_symbols contains candidateBase or candidateBase + detected suffix etc.

                const strictMatch = available_special_symbols.find(s =>
                    s === candidateBase ||
                    s === `${detected_prefix}${candidateBase}${detected_suffix}` ||
                    s === `${candidateBase}${detected_suffix}` // Often indices have suffix but no prefix
                );

                if (strictMatch) {
                    // Found a match! Suggest mapping: MasterBase=StrictMatch
                    // e.g. XAUUSD=GOLD.m
                    newMappings.push(`${masterBase}=${strictMatch}`);
                    break; // Found one candidate for this master base, stop looking
                }
            }
        }
    }

    if (!diffPrefix && !diffSuffix && newMappings.length === 0) {
        return null;
    }

    const handleApply = () => {
        const changes: Partial<SlaveSettingsFormData> = {};
        if (diffPrefix) changes.symbol_prefix = detected_prefix;
        if (diffSuffix) changes.symbol_suffix = detected_suffix;

        if (newMappings.length > 0) {
            const current = formData.symbol_mappings ? formData.symbol_mappings.split(',').filter(s => s).map(s => s.trim()) : [];
            // Combine de-duped
            const combined = Array.from(new Set([...current, ...newMappings])).join(',');
            changes.symbol_mappings = combined;
        }

        onApply(changes);
    };

    return (
        <Alert className="mb-6 border-blue-200 bg-blue-50 dark:bg-blue-900/20 dark:border-blue-800">
            <Wand2 className="h-4 w-4 text-blue-600 dark:text-blue-400" />
            <AlertTitle className="text-blue-800 dark:text-blue-300">
                Recommendation Available
            </AlertTitle>
            <AlertDescription className="text-blue-700 dark:text-blue-400 mt-2">
                <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                    <div className="space-y-1">
                        <p className="text-sm">
                            The EA detected the following symbol settings:
                        </p>
                        <ul className="list-disc list-inside text-sm font-medium ml-2">
                            {diffPrefix && <li>Prefix: <code>{detected_prefix}</code></li>}
                            {diffSuffix && <li>Suffix: <code>{detected_suffix}</code></li>}
                            {newMappings.map(m => (
                                <li key={m}>Mapping: <code>{m}</code></li>
                            ))}
                        </ul>
                    </div>
                    <Button
                        size="sm"
                        variant="outline"
                        className="border-blue-300 hover:bg-blue-100 dark:border-blue-700 dark:hover:bg-blue-800"
                        onClick={handleApply}
                    >
                        Apply Settings
                    </Button>
                </div>
            </AlertDescription>
        </Alert>
    )
}
