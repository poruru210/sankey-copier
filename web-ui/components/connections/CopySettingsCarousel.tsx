'use client';

import { useState } from 'react';
import { ChevronLeft, ChevronRight, RefreshCw, ArrowLeftRight } from 'lucide-react';
import { BrokerIcon } from '@/components/BrokerIcon';
import type { CopySettings } from '@/types';

interface CopySettingsCarouselProps {
  accountSettings: CopySettings[];
  content: {
    copySettings: string;
    lotMultiplier: string;
    marginRatio: string;
    reverseTrade: string;
    symbolRules: string;
    prefix: string;
    suffix: string;
    mappings: string;
    lotFilter: string;
    min: string;
    max: string;
    noSettings: string;
    pageIndicator: string; // "{current} / {total}"
  };
}

/**
 * Carousel component for displaying copy settings from multiple Masters
 * Shows one Master's settings at a time with pagination controls
 */
export function CopySettingsCarousel({
  accountSettings,
  content,
}: CopySettingsCarouselProps) {
  const [currentIndex, setCurrentIndex] = useState(0);

  if (accountSettings.length === 0) {
    return (
      <div className="text-xs text-gray-500 dark:text-gray-400 py-2">
        {content.noSettings}
      </div>
    );
  }

  const currentSetting = accountSettings[currentIndex];
  const hasMultiple = accountSettings.length > 1;

  const handlePrev = () => {
    setCurrentIndex((prev) => (prev > 0 ? prev - 1 : accountSettings.length - 1));
  };

  const handleNext = () => {
    setCurrentIndex((prev) => (prev < accountSettings.length - 1 ? prev + 1 : 0));
  };

  // Parse master account name
  const parseMasterAccount = (accountId: string) => {
    const lastUnderscoreIndex = accountId.lastIndexOf('_');
    if (lastUnderscoreIndex === -1) {
      return { brokerName: accountId, accountNumber: '' };
    }
    return {
      brokerName: accountId.substring(0, lastUnderscoreIndex).replace(/_/g, ' '),
      accountNumber: accountId.substring(lastUnderscoreIndex + 1),
    };
  };

  const masterInfo = parseMasterAccount(currentSetting.master_account);

  // Check if there are any symbol rules configured
  const hasSymbolRules =
    currentSetting.symbol_prefix ||
    currentSetting.symbol_suffix ||
    (currentSetting.symbol_mappings && currentSetting.symbol_mappings.length > 0);

  // Check if there are lot filters
  const hasLotFilter =
    currentSetting.source_lot_min != null || currentSetting.source_lot_max != null;

  return (
    <div className="space-y-1.5">
      {/* Header with title and pagination */}
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide">
          {content.copySettings}
        </span>
        {hasMultiple && (
          <div className="flex items-center gap-1">
            <button
              onClick={handlePrev}
              className="noDrag p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors"
            >
              <ChevronLeft className="w-3.5 h-3.5 text-gray-600 dark:text-gray-400" />
            </button>
            <span className="text-[10px] text-gray-500 dark:text-gray-400 min-w-[32px] text-center">
              {currentIndex + 1} / {accountSettings.length}
            </span>
            <button
              onClick={handleNext}
              className="noDrag p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors"
            >
              <ChevronRight className="w-3.5 h-3.5 text-gray-600 dark:text-gray-400" />
            </button>
          </div>
        )}
      </div>
      <div className="h-px bg-gray-300 dark:bg-gray-600 -mt-1 mb-2"></div>

      {/* Master Account Info */}
      <div className="flex items-center gap-2 mb-2">
        <BrokerIcon brokerName={masterInfo.brokerName} size="sm" />
        <div className="flex-1 min-w-0">
          <div className="text-[10px] text-gray-500 dark:text-gray-400 truncate">
            {masterInfo.brokerName}
          </div>
          {masterInfo.accountNumber && (
            <div className="text-xs font-medium text-gray-900 dark:text-gray-100 truncate">
              {masterInfo.accountNumber}
            </div>
          )}
        </div>
      </div>

      {/* Settings Grid */}
      <div className="grid grid-cols-2 gap-x-2 gap-y-1.5 text-xs">
        {/* Lot Multiplier / Margin Ratio */}
        <div className="flex flex-col min-w-0">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {currentSetting.lot_calculation_mode === 'margin_ratio'
              ? content.marginRatio
              : content.lotMultiplier}
          </span>
          <span className="font-semibold text-gray-900 dark:text-gray-100 truncate">
            {currentSetting.lot_calculation_mode === 'margin_ratio'
              ? 'Auto'
              : `×${currentSetting.lot_multiplier ?? 1}`}
          </span>
        </div>

        {/* Reverse Trade */}
        <div className="flex flex-col min-w-0">
          <span className="text-gray-500 dark:text-gray-500 text-[10px] uppercase tracking-wide">
            {content.reverseTrade}
          </span>
          <span className="font-semibold text-gray-900 dark:text-gray-100 truncate flex items-center gap-1">
            {currentSetting.reverse_trade ? (
              <>
                <ArrowLeftRight className="w-3 h-3 text-orange-500" />
                ON
              </>
            ) : (
              'OFF'
            )}
          </span>
        </div>
      </div>

      {/* Symbol Rules Summary */}
      {hasSymbolRules && (
        <div className="mt-2 pt-2 border-t border-gray-200 dark:border-gray-700">
          <div className="text-[10px] text-gray-500 dark:text-gray-500 uppercase tracking-wide mb-1">
            {content.symbolRules}
          </div>
          <div className="flex flex-wrap gap-1">
            {currentSetting.symbol_prefix && (
              <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300">
                {content.prefix}: {currentSetting.symbol_prefix}
              </span>
            )}
            {currentSetting.symbol_suffix && (
              <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300">
                {content.suffix}: {currentSetting.symbol_suffix}
              </span>
            )}
            {currentSetting.symbol_mappings && currentSetting.symbol_mappings.length > 0 && (
              <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300">
                {content.mappings}: {currentSetting.symbol_mappings.length}
              </span>
            )}
          </div>
        </div>
      )}

      {/* Lot Filter Summary */}
      {hasLotFilter && (
        <div className="mt-2 pt-2 border-t border-gray-200 dark:border-gray-700">
          <div className="text-[10px] text-gray-500 dark:text-gray-500 uppercase tracking-wide mb-1">
            {content.lotFilter}
          </div>
          <div className="flex gap-2 text-[10px]">
            {currentSetting.source_lot_min != null && (
              <span className="text-gray-600 dark:text-gray-400">
                {content.min}: {currentSetting.source_lot_min}
              </span>
            )}
            {currentSetting.source_lot_max != null && (
              <span className="text-gray-600 dark:text-gray-400">
                {content.max}: {currentSetting.source_lot_max}
              </span>
            )}
          </div>
        </div>
      )}

      {/* Pagination dots for mobile */}
      {hasMultiple && (
        <div className="flex justify-center gap-1 pt-2">
          {accountSettings.map((_, idx) => (
            <button
              key={idx}
              onClick={() => setCurrentIndex(idx)}
              className={`noDrag w-1.5 h-1.5 rounded-full transition-colors ${
                idx === currentIndex
                  ? 'bg-blue-500'
                  : 'bg-gray-300 dark:bg-gray-600 hover:bg-gray-400 dark:hover:bg-gray-500'
              }`}
            />
          ))}
        </div>
      )}
    </div>
  );
}
