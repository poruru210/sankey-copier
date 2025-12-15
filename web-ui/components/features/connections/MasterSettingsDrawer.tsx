'use client';

// MasterSettingsDrawer - Drawer component for editing Master account settings
// Opens when user clicks on a Master node in the connections view
// Contains symbol_prefix and symbol_suffix settings for the Master EA

import { useState, useEffect, useRef } from 'react';
import { useIntlayer } from 'next-intlayer';
import { useAtomValue } from 'jotai';
import { Drawer, DrawerContent, DrawerHeader, DrawerTitle, DrawerFooter } from '@/components/ui/drawer';
import { useMediaQuery } from '@/hooks/useMediaQuery';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { BrokerIcon } from '@/components/ui/BrokerIcon';
import { AlertCircle, CheckCircle, Loader2, ChevronRight } from 'lucide-react';
import { apiClientAtom } from '@/lib/atoms/site';
import { DRAWER_SIZE_SETTINGS } from '@/lib/ui-constants';
import {
  DrawerSection,
  DrawerSectionHeader,
  DrawerSectionContent,
  DrawerInfoCard,
  DrawerFormField,
} from '@/components/ui/drawer-section';
import { Caption } from '@/components/ui/typography';
import { SlaveSettingsDrawer } from '@/components/features/settings/SlaveSettingsDrawer';
import type { TradeGroup, MasterSettings, EaConnection, TradeGroupMember } from '@/types';

interface MasterSettingsDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  masterAccount: string;
  connection?: EaConnection;
}

export function MasterSettingsDrawer({
  open,
  onOpenChange,
  masterAccount,
  connection,
}: MasterSettingsDrawerProps) {
  const content = useIntlayer('settings-dialog');
  const apiClient = useAtomValue(apiClientAtom);

  // Responsive: right drawer for desktop, bottom drawer for mobile
  const isDesktop = useMediaQuery('(min-width: 768px)');
  const side = isDesktop ? 'right' : 'bottom';

  const [tradeGroup, setTradeGroup] = useState<TradeGroup | null>(null);
  const [members, setMembers] = useState<TradeGroupMember[]>([]);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);
  const timerRef = useRef<NodeJS.Timeout | null>(null);

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, []);

  const [formData, setFormData] = useState({
    symbol_prefix: '',
    symbol_suffix: '',
  });

  // Nested drawer state for slave settings
  const [selectedMember, setSelectedMember] = useState<TradeGroupMember | null>(null);
  const [slaveDrawerOpen, setSlaveDrawerOpen] = useState(false);

  // Fetch trade group data and members when drawer opens
  useEffect(() => {
    const fetchData = async () => {
      if (!open || !masterAccount || !apiClient) return;

      setLoading(true);
      setMessage(null);
      try {
        // Fetch trade group settings and members in parallel
        const [groupData, membersData] = await Promise.all([
          apiClient.getTradeGroup(masterAccount).catch(() => null),
          apiClient.listTradeGroupMembers(masterAccount).catch(() => []),
        ]);

        if (groupData) {
          setTradeGroup(groupData);
          setFormData({
            symbol_prefix: groupData.master_settings.symbol_prefix || '',
            symbol_suffix: groupData.master_settings.symbol_suffix || '',
          });
        } else {
          setTradeGroup(null);
          setFormData({
            symbol_prefix: '',
            symbol_suffix: '',
          });
        }
        setMembers(membersData || []);
      } catch (err) {
        // TradeGroup may not exist yet - that's OK
        console.log('No existing TradeGroup for', masterAccount);
        setTradeGroup(null);
        setMembers([]);
        setFormData({
          symbol_prefix: '',
          symbol_suffix: '',
        });
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, [open, masterAccount, apiClient]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setMessage(null);

    try {
      const settings: MasterSettings = {
        enabled: tradeGroup?.master_settings.enabled ?? true,
        symbol_prefix: formData.symbol_prefix || null,
        symbol_suffix: formData.symbol_suffix || null,
        config_version: tradeGroup?.master_settings.config_version || 0,
      };

      await apiClient.updateTradeGroupSettings(masterAccount, settings);
      setMessage({ type: 'success', text: content.settingsSavedSuccess.value });

      // Close after short delay
      timerRef.current = setTimeout(() => {
        onOpenChange(false);
      }, 1000);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : content.settingsSaveFailed.value;
      setMessage({ type: 'error', text: errorMessage });
      console.error('Error updating master settings:', err);
    } finally {
      setSaving(false);
    }
  };

  // Split account name into broker name and account number
  const splitAccountName = (accountName: string) => {
    const lastUnderscoreIndex = accountName.lastIndexOf('_');
    if (lastUnderscoreIndex === -1) {
      return { brokerName: accountName, accountNumber: '' };
    }
    return {
      brokerName: accountName.substring(0, lastUnderscoreIndex).replace(/_/g, ' '),
      accountNumber: accountName.substring(lastUnderscoreIndex + 1),
    };
  };

  const accountInfo = splitAccountName(masterAccount);

  return (
    <Drawer open={open} onOpenChange={onOpenChange} direction={side}>
      <DrawerContent
        side={side}
        className={cn(
          'overflow-hidden p-6',
          isDesktop
            ? `h-full w-full ${DRAWER_SIZE_SETTINGS.desktop}`
            : DRAWER_SIZE_SETTINGS.mobile
        )}
      >
        <DrawerHeader className={isDesktop ? 'mt-0' : ''}>
          <DrawerTitle>{content.masterSettingsTitle.value}</DrawerTitle>
        </DrawerHeader>

        <form onSubmit={handleSubmit} className="flex flex-col flex-1 overflow-hidden">
          <div className="flex-1 overflow-y-auto pr-2 space-y-6">
            {/* Account Display */}
            <DrawerInfoCard>
              <div className="flex items-center gap-3">
                <BrokerIcon brokerName={accountInfo.brokerName} size="md" />
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-sm truncate">
                    {accountInfo.brokerName}
                  </div>
                  {accountInfo.accountNumber && (
                    <div className="text-xs text-muted-foreground">
                      {accountInfo.accountNumber}
                    </div>
                  )}
                </div>
              </div>
            </DrawerInfoCard>

            {loading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin" />
              </div>
            ) : (
              <>
                {/* Symbol Rules Section */}
                <DrawerSection bordered>
                  <DrawerSectionHeader
                    title={content.symbolFiltersGlobalTitle.value}
                    description={content.symbolFiltersGlobalDescription.value}
                  />
                  <DrawerSectionContent>
                    {/* Symbol Prefix */}
                    <DrawerFormField
                      label={content.symbolPrefix.value}
                      description={content.masterSymbolPrefixDescription.value}
                      htmlFor="master_symbol_prefix"
                    >
                      <Input
                        id="master_symbol_prefix"
                        type="text"
                        placeholder={content.symbolPrefixPlaceholder.value}
                        value={formData.symbol_prefix}
                        onChange={(e) => setFormData({ ...formData, symbol_prefix: e.target.value })}
                        disabled={saving}
                      />
                    </DrawerFormField>

                    {/* Symbol Suffix */}
                    <DrawerFormField
                      label={content.symbolSuffix.value}
                      description={content.masterSymbolSuffixDescription.value}
                      htmlFor="master_symbol_suffix"
                    >
                      <Input
                        id="master_symbol_suffix"
                        type="text"
                        placeholder={content.symbolSuffixPlaceholder.value}
                        value={formData.symbol_suffix}
                        onChange={(e) => setFormData({ ...formData, symbol_suffix: e.target.value })}
                        disabled={saving}
                      />
                    </DrawerFormField>
                  </DrawerSectionContent>
                </DrawerSection>

                {/* Connected Slaves Section */}
                <DrawerSection bordered>
                  <DrawerSectionHeader
                    title={content.connectedSlavesTitle.value}
                  />
                  <DrawerSectionContent>
                    {members.length === 0 ? (
                      <Caption>{content.noConnectedSlaves.value}</Caption>
                    ) : (
                      <div className="space-y-2">
                        {members.map((member) => {
                          const slaveInfo = splitAccountName(member.slave_account);
                          return (
                            <button
                              key={member.id}
                              type="button"
                              onClick={() => {
                                setSelectedMember(member);
                                setSlaveDrawerOpen(true);
                              }}
                              className="flex items-center gap-2 p-2 rounded-md bg-muted/50 w-full text-left hover:bg-muted transition-colors"
                            >
                              <BrokerIcon brokerName={slaveInfo.brokerName} size="sm" />
                              <div className="flex-1 min-w-0">
                                <div className="text-sm truncate">
                                  {slaveInfo.brokerName}
                                </div>
                                {slaveInfo.accountNumber && (
                                  <div className="text-xs text-muted-foreground">
                                    {slaveInfo.accountNumber}
                                  </div>
                                )}
                              </div>
                              <ChevronRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                            </button>
                          );
                        })}
                      </div>
                    )}
                  </DrawerSectionContent>
                </DrawerSection>

                {/* Message Display */}
                {message && (
                  <div
                    className={cn(
                      'px-4 py-3 rounded-lg flex items-center gap-2 text-sm',
                      message.type === 'success'
                        ? 'bg-green-500/10 border border-green-500 text-green-600 dark:text-green-400'
                        : 'bg-destructive/10 border border-destructive text-destructive'
                    )}
                  >
                    {message.type === 'success' ? (
                      <CheckCircle className="h-4 w-4" />
                    ) : (
                      <AlertCircle className="h-4 w-4" />
                    )}
                    {message.text}
                  </div>
                )}
              </>
            )}
          </div>

          <DrawerFooter className="flex-shrink-0 pt-4 border-t mt-4">
            <div className="flex w-full justify-end items-center gap-2">
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                {content.cancel.value}
              </Button>
              <Button type="submit" disabled={loading || saving}>
                {saving ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin mr-2" />
                    {content.saving.value}
                  </>
                ) : (
                  content.save.value
                )}
              </Button>
            </div>
          </DrawerFooter>
        </form>
      </DrawerContent>

      {/* Nested Slave Settings Drawer */}
      <SlaveSettingsDrawer
        open={slaveDrawerOpen}
        onOpenChange={setSlaveDrawerOpen}
        member={selectedMember}
        masterAccount={masterAccount}
        onSaved={() => {
          // Refresh members list after save
          if (apiClient) {
            apiClient.listTradeGroupMembers(masterAccount)
              .then(data => setMembers(data || []))
              .catch(() => { });
          }
        }}
      />
    </Drawer>
  );
}
