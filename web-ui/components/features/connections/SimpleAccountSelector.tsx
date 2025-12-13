'use client';

import React, { useMemo } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectTrigger,
  SelectItemCustom,
} from '@/components/ui/select';
import { BrokerIcon } from '@/components/ui/BrokerIcon';
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

  // Get default placeholder based on filter type
  const defaultPlaceholder = placeholder ||
    (filterType === 'Master' ? strings.selectMasterAccount : strings.selectSlaveAccount);

  // Get no accounts message
  const noAccountsMessage = filterType === 'Master'
    ? strings.noConnectedMasterAccounts
    : strings.noConnectedSlaveAccounts;

  const testId = filterType === 'Master'
    ? 'master-selector'
    : filterType === 'Slave'
    ? 'slave-selector'
    : undefined;

  // Find selected connection for display
  const selectedConnection = sortedConnections.find((conn) => conn.account_id === value);
  const selectedDisplay = selectedConnection
    ? {
        brokerName: selectedConnection.broker,
        accountNumber: selectedConnection.account_number.toString(),
      }
    : null;

  return (
    <div className="space-y-2">
      {label && (
        <Label>
          {label}
        </Label>
      )}
      <Select value={value} onValueChange={onChange} required={required}>
        <SelectTrigger className="h-auto py-2" data-testid={testId}>
          <div className="flex items-center gap-2 w-full overflow-hidden">
            {selectedDisplay ? (
              <>
                <span className="text-sm flex-shrink-0">{getStatusEmoji(selectedConnection!.status)}</span>
                <BrokerIcon brokerName={selectedDisplay.brokerName} size="sm" />
                <div className="flex-1 min-w-0 text-left flex flex-col">
                  <div className="font-normal text-foreground text-sm truncate leading-tight">
                    {selectedDisplay.brokerName}
                  </div>
                  {selectedDisplay.accountNumber && (
                    <div className="text-xs text-muted-foreground truncate leading-tight">
                      {selectedDisplay.accountNumber}
                    </div>
                  )}
                </div>
              </>
            ) : (
              <span className="text-muted-foreground">{defaultPlaceholder}</span>
            )}
          </div>
        </SelectTrigger>
        <SelectContent>
          {sortedConnections.map((conn) => {
            const brokerName = conn.broker;
            const accountNumber = conn.account_number.toString();
            return (
              <SelectItemCustom
                key={conn.account_id}
                value={conn.account_id}
                textValue={`${brokerName} ${accountNumber}`}
              >
                <div className="flex items-center gap-2 flex-1 min-w-0">
                  <span className="text-sm">{getStatusEmoji(conn.status)}</span>
                  <BrokerIcon brokerName={brokerName} size="sm" />
                  <div className="flex-1 min-w-0">
                    <div className="font-normal text-foreground text-sm truncate">
                      {brokerName}
                    </div>
                    {accountNumber && (
                      <div className="text-xs text-muted-foreground truncate">
                        {accountNumber}
                      </div>
                    )}
                  </div>
                </div>
              </SelectItemCustom>
            );
          })}
        </SelectContent>
      </Select>

      {filteredConnections.length === 0 && (
        <p className="text-sm text-muted-foreground">
          {noAccountsMessage}
        </p>
      )}
    </div>
  );
}
