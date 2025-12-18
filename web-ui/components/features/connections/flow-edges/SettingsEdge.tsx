'use client';

import React, { memo } from 'react';
import {
  BaseEdge,
  EdgeProps,
  EdgeLabelRenderer,
  getBezierPath,
  Edge,
} from '@xyflow/react';
import { Trash2, Settings } from 'lucide-react';
import { useIntlayer } from 'next-intlayer';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import type { CopySettings } from '@/types';

export interface SettingsEdgeData {
  setting: CopySettings;
  onEditSetting: (setting: CopySettings) => void;
  onDeleteSetting: (setting: CopySettings) => void;
}

export type SettingsEdgeType = Edge<SettingsEdgeData & Record<string, unknown>, 'settingsEdge'>;

/**
 * Displays connection label and a trash icon for deletion
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
  const content = useIntlayer('settings-dialog');
  const [showDeleteConfirm, setShowDeleteConfirm] = React.useState(false);
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
  const onDeleteSetting = data?.onDeleteSetting;

  // Build label text with copy settings
  const labelParts: string[] = [];
  if (setting?.lot_multiplier !== null && setting?.lot_multiplier !== undefined) {
    labelParts.push(`×${setting.lot_multiplier}`);
  }
  if (setting?.reverse_trade) {
    labelParts.push('⇄');
  }
  const labelText = labelParts.length > 0 ? labelParts.join(' ') : '';

  // Determine edge state based on status:
  // 0 = DISABLED (gray), 1 = ENABLED/waiting (yellow), 2 = CONNECTED (green)
  const runtimeStatus = setting?.status ?? 0;
  const isConnected = runtimeStatus === 2;
  const isEnabled = runtimeStatus === 1;


  const handleSettingsClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (setting && onEditSetting) {
      onEditSetting(setting);
    }
  };

  const handleDeleteClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowDeleteConfirm(true);
  };

  const confirmDelete = () => {
    if (setting && onDeleteSetting) {
      onDeleteSetting(setting);
    }
    setShowDeleteConfirm(false);
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
                className={`text-xs font-semibold ${isConnected
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
              onClick={handleDeleteClick}
              className="p-1 hover:bg-red-100 dark:hover:bg-red-900/30 rounded transition-colors text-red-600 dark:text-red-400"
              title={content.delete.value}
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </EdgeLabelRenderer>
      <AlertDialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <AlertDialogContent onClick={(e) => e.stopPropagation()}>
          <AlertDialogHeader>
            <AlertDialogTitle>{content.deleteConfirmTitle.value}</AlertDialogTitle>
            <AlertDialogDescription>
              {content.deleteConfirmDescription.value}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={(e) => e.stopPropagation()}>{content.cancel.value}</AlertDialogCancel>
            <AlertDialogAction
              onClick={(e) => {
                e.stopPropagation();
                confirmDelete();
              }}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {content.delete.value}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
});

SettingsEdge.displayName = 'SettingsEdge';
