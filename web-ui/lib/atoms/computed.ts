import { atom } from 'jotai';
import type { AccountInfo } from '@/types';
import {
  connectionsAtom,
  settingsAtom,
  tradeGroupsAtom,
  localizationAtom,
} from './data';
import {
  expandedSourceIdsAtom,
  expandedReceiverIdsAtom,
  disabledReceiverIdsAtom,
} from './ui';

export const sourceAccountsAtom = atom<AccountInfo[]>((get) => {
  const settings = get(settingsAtom);
  const connections = get(connectionsAtom);
  const tradeGroups = get(tradeGroupsAtom);
  const expandedSourceIds = get(expandedSourceIdsAtom);
  const content = get(localizationAtom);

  const sourceMap = new Map<string, AccountInfo>();

  settings.forEach((setting) => {
    if (!sourceMap.has(setting.master_account)) {
      const conn = connections.find((c) => c.account_id === setting.master_account);
      const isOnline = conn?.is_online ?? (conn?.status === 'Online' || false);
      const tradeGroup = tradeGroups.find((tg) => tg.id === setting.master_account);
      const isEnabled = tradeGroup?.master_settings.enabled ?? true;
      const isExpanded = expandedSourceIds.includes(setting.master_account);
      const masterRuntimeStatus = tradeGroup?.master_runtime_status;
      const masterWarningCodes = tradeGroup?.master_warning_codes ?? [];
      const hasAutoTradingWarning = masterWarningCodes.includes('master_auto_trading_disabled');
      const runtimeStatus = masterRuntimeStatus ?? 0;
      const isActive = runtimeStatus === 2;
      const hasWarning = hasAutoTradingWarning && isOnline;
      const hasError = false;
      const errorMsg = hasWarning ? content.autoTradingDisabled : '';

      sourceMap.set(setting.master_account, {
        id: setting.master_account,
        name: setting.master_account,
        accountType: 'master',
        platform: conn?.platform,
        isOnline,
        isEnabled,
        isActive,
        hasError,
        hasWarning,
        errorMsg,
        isExpanded,
        masterRuntimeStatus,
        runtimeStatus,
        masterIntentEnabled: isEnabled,
      });
    }
  });

  return Array.from(sourceMap.values());
});

export const receiverAccountsAtom = atom<AccountInfo[]>((get) => {
  const settings = get(settingsAtom);
  const connections = get(connectionsAtom);
  const expandedReceiverIds = get(expandedReceiverIdsAtom);
  const disabledReceiverIds = get(disabledReceiverIdsAtom);
  const content = get(localizationAtom);

  const receiverMap = new Map<string, AccountInfo>();
  const receiverRuntimeStatuses = new Map<string, number[]>();

  settings.forEach((setting) => {
    const runtimeStatusValue = setting.status ?? 0;
    const existingStatuses = receiverRuntimeStatuses.get(setting.slave_account) ?? [];
    existingStatuses.push(runtimeStatusValue);
    receiverRuntimeStatuses.set(setting.slave_account, existingStatuses);

    if (!receiverMap.has(setting.slave_account)) {
      const conn = connections.find((c) => c.account_id === setting.slave_account);
      const isOnline = conn?.is_online ?? (conn?.status === 'Online' || false);
      const intentEnabled = setting.enabled_flag ?? (setting.status !== 0);
      const isManuallyDisabled = disabledReceiverIds.includes(setting.slave_account);
      const isEnabled = isManuallyDisabled ? false : intentEnabled;
      const isExpanded = expandedReceiverIds.includes(setting.slave_account);
      const slaveWarningCodes = setting.warning_codes ?? [];
      const hasAutoTradingWarning = slaveWarningCodes.includes('slave_auto_trading_disabled');
      const hasWarning = hasAutoTradingWarning && isOnline;

      receiverMap.set(setting.slave_account, {
        id: setting.slave_account,
        name: setting.slave_account,
        accountType: 'slave',
        platform: conn?.platform,
        isOnline,
        isEnabled,
        isActive: false, // Updated below
        hasError: false,
        hasWarning,
        errorMsg: hasWarning ? content.autoTradingDisabled : '',
        isExpanded,
        slaveIntentEnabled: intentEnabled,
        runtimeStatus: runtimeStatusValue,
      });
    } else {
      const existing = receiverMap.get(setting.slave_account)!;
      const currentIntentEnabled = setting.enabled_flag ?? (setting.status !== 0);

      if (currentIntentEnabled) {
        existing.slaveIntentEnabled = true;
        const isManuallyDisabled = disabledReceiverIds.includes(setting.slave_account);
        existing.isEnabled = isManuallyDisabled ? false : true;
      }

      const slaveWarningCodes = setting.warning_codes ?? [];
      const hasAutoTradingWarning = slaveWarningCodes.includes('slave_auto_trading_disabled');
      if (hasAutoTradingWarning && existing.isOnline) {
        existing.hasWarning = true;
        existing.errorMsg = content.autoTradingDisabled;
      }
    }
  });

  const accounts = Array.from(receiverMap.values());

  accounts.forEach((receiver) => {
    const statuses = receiverRuntimeStatuses.get(receiver.id) ?? [];
    let runtimeStatus = statuses.length > 0 ? Math.max(...statuses) : 0;

    const receiverSettings = settings.filter((s) => s.slave_account === receiver.id);
    const hasAutoTradingDisabled = receiverSettings.some((s) =>
      (s.warning_codes ?? []).includes('slave_auto_trading_disabled')
    );

    if (hasAutoTradingDisabled) {
      runtimeStatus = 0;
    }

    receiver.runtimeStatus = runtimeStatus;
    receiver.isActive = receiver.isOnline && receiver.isEnabled && runtimeStatus === 2;
  });

  return accounts;
});
