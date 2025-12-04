'use client';

import React, { memo } from 'react';
import {
  BaseEdge,
  EdgeProps,
  EdgeLabelRenderer,
  getBezierPath,
  Edge,
} from '@xyflow/react';
import { Settings } from 'lucide-react';
import { useIntlayer } from 'next-intlayer';
import type { CopySettings } from '@/types';

export interface SettingsEdgeData {
  setting: CopySettings;
  onEditSetting: (setting: CopySettings) => void;
}

export type SettingsEdgeType = Edge<SettingsEdgeData & Record<string, unknown>, 'settingsEdge'>;

/**
 * Custom edge component with a settings button
 * Displays connection label and a gear icon for editing
 */
export const SettingsEdge = memo(({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
  style,
  markerEnd,
}: EdgeProps<SettingsEdgeType>) => {
  const content = useIntlayer('settings-edge');
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const setting = data?.setting;
  const onEditSetting = data?.onEditSetting;

  // Build label text with copy settings
  const labelParts: string[] = [];
  if (setting?.lot_multiplier !== null && setting?.lot_multiplier !== undefined) {
    labelParts.push(`×${setting.lot_multiplier}`);
  }
  if (setting?.reverse_trade) {
    labelParts.push('⇄');
  }
  const labelText = labelParts.length > 0 ? labelParts.join(' ') : '';

  // Determine edge state based on runtime_status:
  // 0 = DISABLED (gray), 1 = ENABLED/waiting (yellow), 2 = CONNECTED (green)
  const runtimeStatus = setting?.runtime_status ?? setting?.status;
  const isConnected = runtimeStatus === 2;
  const isEnabled = runtimeStatus === 1;

  const handleSettingsClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (setting && onEditSetting) {
      onEditSetting(setting);
    }
  };

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        style={style}
        markerEnd={markerEnd}
      />
      <EdgeLabelRenderer>
        <div
          style={{
            position: 'absolute',
            transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
            pointerEvents: 'all',
          }}
          className="nodrag nopan"
        >
          <div className="flex items-center gap-1 bg-white dark:bg-gray-800 rounded-md shadow-sm border border-gray-200 dark:border-gray-700 px-2 py-1">
            {labelText && (
              <span
                className={`text-xs font-semibold ${
                  isConnected
                    ? 'text-green-600 dark:text-green-400'
                    : isEnabled
                    ? 'text-yellow-600 dark:text-yellow-400'
                    : 'text-gray-500 dark:text-gray-400'
                }`}
              >
                {labelText}
              </span>
            )}
            <button
              onClick={handleSettingsClick}
              className="p-1 hover:bg-blue-100 dark:hover:bg-blue-900/30 rounded transition-colors text-blue-600 dark:text-blue-400"
              title={content.connectionSettingsTitle}
            >
              <Settings className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </EdgeLabelRenderer>
    </>
  );
});

SettingsEdge.displayName = 'SettingsEdge';
