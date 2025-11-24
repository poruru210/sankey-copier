'use client';

import React, { useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Label } from '@/components/ui/label';
import { formatRelativeTime } from '@/lib/time-utils';
import { extractBrokerName } from '@/lib/brokerIcons';
import type { EaConnection } from '@/types';

interface SimpleAccountSelectorProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  connections: EaConnection[];
  filterType?: 'Master' | 'Slave';
  placeholder?: string;
  required?: boolean;
}

export function SimpleAccountSelector({
  label,
  value,
  onChange,
  connections,
  filterType,
  placeholder,
  required = false,
}: SimpleAccountSelectorProps) {
  const content = useIntlayer('settings-dialog');

  // Convert all Intlayer strings to plain strings using useMemo
  // Try accessing .value property or use String() constructor
  const strings = useMemo(() => ({
    selectMasterAccount: String(content.selectMasterAccount?.value ?? content.selectMasterAccount),
    selectSlaveAccount: String(content.selectSlaveAccount?.value ?? content.selectSlaveAccount),
    connectedMasterAccounts: String(content.connectedMasterAccounts?.value ?? content.connectedMasterAccounts),
    connectedSlaveAccounts: String(content.connectedSlaveAccounts?.value ?? content.connectedSlaveAccounts),
    timeoutAccounts: String(content.timeoutAccounts?.value ?? content.timeoutAccounts),
    offlineAccounts: String(content.offlineAccounts?.value ?? content.offlineAccounts),
    noConnectedMasterAccounts: String(content.noConnectedMasterAccounts?.value ?? content.noConnectedMasterAccounts),
    noConnectedSlaveAccounts: String(content.noConnectedSlaveAccounts?.value ?? content.noConnectedSlaveAccounts),
    positionsLabel: String(content.positionsLabel?.value ?? content.positionsLabel),
    lastConnectionLabel: String(content.lastConnectionLabel?.value ?? content.lastConnectionLabel),
    timeAgoSeconds: String(content.timeAgoSeconds?.value ?? content.timeAgoSeconds),
    timeAgoMinutes: String(content.timeAgoMinutes?.value ?? content.timeAgoMinutes),
    timeAgoHours: String(content.timeAgoHours?.value ?? content.timeAgoHours),
    timeAgoDays: String(content.timeAgoDays?.value ?? content.timeAgoDays),
  }), [content]);

  // Filter connections by type if specified
  const filteredConnections = filterType
    ? connections.filter((conn) => conn.ea_type === filterType)
    : connections;

  // Sort connections: Online first, then Timeout, then Offline
  const sortedConnections = useMemo(() => {
    return [...filteredConnections].sort((a, b) => {
      const statusOrder = { 'Online': 0, 'Timeout': 1, 'Offline': 2 };
      return statusOrder[a.status as keyof typeof statusOrder] - statusOrder[b.status as keyof typeof statusOrder];
    });
  }, [filteredConnections]);

  // Get status emoji
  const getStatusEmoji = (status: string) => {
    switch (status) {
      case 'Online':
        return '🟢';
      case 'Timeout':
        return '🟡';
      case 'Offline':
        return '⚫';
      default:
        return '⚪';
    }
  };

  // Format account display with broker info
  const formatAccountDisplay = (conn: EaConnection) => {
    const emoji = getStatusEmoji(conn.status);
    const brokerName = extractBrokerName(conn.account_name);

    // Format: Status emoji | Broker name | Account ID
    return `${emoji} ${brokerName} - ${conn.account_id}`;
  };

  // Get default placeholder based on filter type
  const defaultPlaceholder = placeholder ||
    (filterType === 'Master' ? strings.selectMasterAccount : strings.selectSlaveAccount);

  // Get no accounts message
  const noAccountsMessage = filterType === 'Master'
    ? strings.noConnectedMasterAccounts
    : strings.noConnectedSlaveAccounts;

  return (
    <div className="space-y-2">
      {label && (
        <Label htmlFor={label}>
          {label}
        </Label>
      )}
      <select
        id={label}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        required={required}
        className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
      >
        <option value="">{defaultPlaceholder}</option>

        {sortedConnections.map((conn) => (
          <option key={conn.account_id} value={conn.account_id}>
            {formatAccountDisplay(conn)}
          </option>
        ))}
      </select>

      {filteredConnections.length === 0 && (
        <p className="text-sm text-muted-foreground">
          {noAccountsMessage}
        </p>
      )}
    </div>
  );
}
