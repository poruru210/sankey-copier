'use client';

import { useMemo } from 'react';
import type { CopySettings, EaConnection } from '@/types';

interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
}

interface ValidationMessages {
  selectMasterAccount: string;
  selectSlaveAccount: string;
  sameAccountError: string;
  lotMultiplierPositive: string;
  lotMultiplierTooSmall: string;
  lotMultiplierTooLarge: string;
  duplicateSettings: string;
  statusEnabled: string;
  statusDisabled: string;
  accountOffline: string;
  accountTimeout: string;
  accountNotInList: string;
  circularReference: string;
}

interface UseSettingsValidationProps {
  masterAccount: string;
  slaveAccount: string;
  lotMultiplier: number;
  existingSettings: CopySettings[];
  connections: EaConnection[];
  currentSettingId?: number; // For edit mode
  messages: ValidationMessages;
}

export function useSettingsValidation({
  masterAccount,
  slaveAccount,
  lotMultiplier,
  existingSettings,
  connections,
  currentSettingId,
  messages,
}: UseSettingsValidationProps): ValidationResult {
  return useMemo(() => {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Check if accounts are selected
    if (!masterAccount) {
      errors.push(messages.selectMasterAccount);
    }

    if (!slaveAccount) {
      errors.push(messages.selectSlaveAccount);
    }

    // Check if same account is selected for both
    if (masterAccount && slaveAccount && masterAccount === slaveAccount) {
      errors.push(messages.sameAccountError);
    }

    // Check lot multiplier
    if (lotMultiplier <= 0) {
      errors.push(messages.lotMultiplierPositive);
    } else if (lotMultiplier < 0.01) {
      warnings.push(messages.lotMultiplierTooSmall);
    } else if (lotMultiplier > 100) {
      warnings.push(messages.lotMultiplierTooLarge);
    }

    // Check for duplicate settings
    if (masterAccount && slaveAccount) {
      const duplicate = existingSettings.find(
        (setting) =>
          setting.master_account === masterAccount &&
          setting.slave_account === slaveAccount &&
          (!currentSettingId || setting.id !== currentSettingId)
      );

      if (duplicate) {
        const status = duplicate.enabled ? messages.statusEnabled : messages.statusDisabled;
        errors.push(
          messages.duplicateSettings
            .replace('{id}', String(duplicate.id))
            .replace('{status}', status)
        );
      }
    }

    // Check connection status
    if (masterAccount) {
      const masterConn = connections.find(
        (conn) => conn.account_id === masterAccount
      );

      if (masterConn) {
        if (masterConn.status === 'Offline') {
          warnings.push(
            messages.accountOffline.replace('{account}', masterAccount)
          );
        } else if (masterConn.status === 'Timeout') {
          warnings.push(
            messages.accountTimeout.replace('{account}', masterAccount)
          );
        }
      } else {
        warnings.push(
          messages.accountNotInList.replace('{account}', masterAccount)
        );
      }
    }

    if (slaveAccount) {
      const slaveConn = connections.find(
        (conn) => conn.account_id === slaveAccount
      );

      if (slaveConn) {
        if (slaveConn.status === 'Offline') {
          warnings.push(
            messages.accountOffline.replace('{account}', slaveAccount)
          );
        } else if (slaveConn.status === 'Timeout') {
          warnings.push(
            messages.accountTimeout.replace('{account}', slaveAccount)
          );
        }
      } else {
        warnings.push(
          messages.accountNotInList.replace('{account}', slaveAccount)
        );
      }
    }

    // Check for potential circular references
    if (masterAccount && slaveAccount) {
      const reverseConnection = existingSettings.find(
        (setting) =>
          setting.master_account === slaveAccount &&
          setting.slave_account === masterAccount &&
          setting.enabled
      );

      if (reverseConnection) {
        warnings.push(
          messages.circularReference
            .replace('{slave}', slaveAccount)
            .replace('{master}', masterAccount)
        );
      }
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
    };
  }, [
    masterAccount,
    slaveAccount,
    lotMultiplier,
    existingSettings,
    connections,
    currentSettingId,
    messages,
  ]);
}
