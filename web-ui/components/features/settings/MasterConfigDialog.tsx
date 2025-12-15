/**
 * Master EA Configuration Dialog
 *
 * Allows configuring symbol prefix/suffix for a Master EA account.
 * These settings apply globally to all Slaves in this Master's trade groups.
 */

'use client';

import { useState, useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
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
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { BrokerIcon } from '@/components/ui/BrokerIcon';
import type { MasterConfig, EaConnection } from '@/types';
import { useMasterConfig } from '@/hooks/useMasterConfig';

interface MasterConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  masterAccount: EaConnection;
}

export function MasterConfigDialog({
  open,
  onOpenChange,
  masterAccount,
}: MasterConfigDialogProps) {
  const content = useIntlayer('master-config-dialog');
  const { getMasterConfig, updateMasterConfig, deleteMasterConfig } =
    useMasterConfig();

  const [formData, setFormData] = useState({
    symbol_prefix: '',
    symbol_suffix: '',
  });
  const [currentConfig, setCurrentConfig] = useState<MasterConfig | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load existing configuration when dialog opens
  useEffect(() => {
    if (open && masterAccount) {
      loadConfig();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, masterAccount.account_id]);

  const loadConfig = async () => {
    try {
      setLoading(true);
      setError(null);
      const config = await getMasterConfig(masterAccount.account_id);
      setCurrentConfig(config);

      if (config) {
        setFormData({
          symbol_prefix: config.symbol_prefix || '',
          symbol_suffix: config.symbol_suffix || '',
        });
      } else {
        // No config exists yet, use empty values
        setFormData({
          symbol_prefix: '',
          symbol_suffix: '',
        });
      }
    } catch (err) {
      const errorMsg =
        err instanceof Error ? err.message : content.loadError.value;
      setError(errorMsg);
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    try {
      setLoading(true);
      setError(null);

      // Convert empty strings to null for API
      const configData = {
        symbol_prefix: formData.symbol_prefix || null,
        symbol_suffix: formData.symbol_suffix || null,
      };

      await updateMasterConfig(masterAccount.account_id, configData);
      onOpenChange(false);
    } catch (err) {
      const errorMsg =
        err instanceof Error ? err.message : content.saveError.value;
      setError(errorMsg);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async () => {
    try {
      setLoading(true);
      setError(null);
      await deleteMasterConfig(masterAccount.account_id);
      setShowDeleteConfirm(false);
      onOpenChange(false);
    } catch (err) {
      const errorMsg =
        err instanceof Error ? err.message : content.deleteError.value;
      setError(errorMsg);
    } finally {
      setLoading(false);
    }
  };

  // Split account name into broker name and account number
  const splitAccountName = (accountName: string) => {
    const lastUnderscoreIndex = accountName.lastIndexOf('_');
    if (lastUnderscoreIndex === -1) {
      return { brokerName: accountName, accountNumber: '' };
    }
    return {
      brokerName: accountName
        .substring(0, lastUnderscoreIndex)
        .replace(/_/g, ' '),
      accountNumber: accountName.substring(lastUnderscoreIndex + 1),
    };
  };

  const accountInfo = splitAccountName(masterAccount.account_id);

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{content.title}</DialogTitle>
          </DialogHeader>

          <form onSubmit={handleSubmit} className="space-y-4">
            {/* Account Display */}
            <div className="space-y-3">
              <div className="space-y-1">
                <h3 className="text-sm font-medium flex items-center gap-2">
                  <span className="text-lg">📊</span>
                  {content.masterAccountLabel}
                </h3>
              </div>
              <div className="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                <div className="flex items-center gap-3">
                  <BrokerIcon brokerName={accountInfo.brokerName} size="sm" />
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-xs text-gray-900 dark:text-gray-100 truncate">
                      {accountInfo.brokerName}
                    </div>
                    {accountInfo.accountNumber && (
                      <div className="text-xs text-gray-600 dark:text-gray-400">
                        {accountInfo.accountNumber}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            </div>

            {/* Error Display */}
            {error && (
              <div className="bg-destructive/10 border border-destructive text-destructive px-3 py-2 rounded-lg text-sm">
                {error}
              </div>
            )}

            {/* Symbol Transformation Settings */}
            <div className="space-y-4">
              <div className="space-y-1">
                <h3 className="text-sm font-medium flex items-center gap-2">
                  <span className="text-lg">🔍</span>
                  {content.symbolTransformationTitle}
                </h3>
                <p className="text-xs text-muted-foreground">
                  {content.symbolTransformationDescription}
                </p>
              </div>

              {/* Symbol Prefix */}
              <div>
                <Label htmlFor="symbol_prefix">{content.symbolPrefix}</Label>
                <Input
                  id="symbol_prefix"
                  type="text"
                  placeholder={content.symbolPrefixPlaceholder.value}
                  value={formData.symbol_prefix}
                  onChange={(e) =>
                    setFormData({ ...formData, symbol_prefix: e.target.value })
                  }
                  disabled={loading}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  {content.symbolPrefixDescription}
                </p>
              </div>

              {/* Symbol Suffix */}
              <div>
                <Label htmlFor="symbol_suffix">{content.symbolSuffix}</Label>
                <Input
                  id="symbol_suffix"
                  type="text"
                  placeholder={content.symbolSuffixPlaceholder.value}
                  value={formData.symbol_suffix}
                  onChange={(e) =>
                    setFormData({ ...formData, symbol_suffix: e.target.value })
                  }
                  disabled={loading}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  {content.symbolSuffixDescription}
                </p>
              </div>
            </div>

            <DialogFooter className="flex w-full justify-between items-center">
              <div>
                {currentConfig && (
                  <Button
                    type="button"
                    variant="destructive"
                    onClick={() => setShowDeleteConfirm(true)}
                    disabled={loading}
                  >
                    {content.delete}
                  </Button>
                )}
              </div>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                  disabled={loading}
                >
                  {content.cancel}
                </Button>
                <Button type="submit" disabled={loading}>
                  {loading ? content.saving : content.save}
                </Button>
              </div>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{content.deleteConfirmTitle}</AlertDialogTitle>
            <AlertDialogDescription>
              {content.deleteConfirmDescription}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={loading}>{content.cancel}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              disabled={loading}
            >
              {loading ? content.deleting : content.delete}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
