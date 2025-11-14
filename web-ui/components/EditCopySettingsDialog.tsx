'use client';

import { useState, useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
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
import { Checkbox } from '@/components/ui/checkbox';
import { BrokerIcon } from '@/components/BrokerIcon';
import type { CopySettings } from '@/types';

interface EditCopySettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (data: CopySettings) => void;
  onDelete: (data: CopySettings) => void;
  setting: CopySettings;
}

export function EditCopySettingsDialog({
  open,
  onOpenChange,
  onSave,
  onDelete,
  setting
}: EditCopySettingsDialogProps) {
  const content = useIntlayer('settings-dialog');

  const [formData, setFormData] = useState({
    lot_multiplier: 1.0,
    reverse_trade: false,
  });

  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  useEffect(() => {
    if (setting) {
      setFormData({
        lot_multiplier: setting.lot_multiplier || 1.0,
        reverse_trade: setting.reverse_trade,
      });
    }
  }, [setting, open]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    onSave({
      ...setting,
      ...formData,
    });
    onOpenChange(false);
  };

  const handleDelete = () => {
    onDelete(setting);
    onOpenChange(false);
    setShowDeleteConfirm(false);
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

  const masterAccount = splitAccountName(setting.master_account);
  const slaveAccount = splitAccountName(setting.slave_account);

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-lg max-h-[90vh] flex flex-col">
          <DialogHeader>
            <DialogTitle>{content.editTitle.value}</DialogTitle>
          </DialogHeader>
          <form onSubmit={handleSubmit} className="flex flex-col overflow-hidden">
            <div className="space-y-4 overflow-y-auto pr-2">
              {/* Account Display */}
              <div className="space-y-3">
                <div className="space-y-1">
                  <h3 className="text-sm font-medium flex items-center gap-2">
                    <span className="text-lg">🔗</span>
                    {content.connectionLabel.value}
                  </h3>
                </div>
                <div className="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                  <div className="flex items-center gap-3">
                    {/* Master Account */}
                    <div className="flex items-center gap-2 flex-1">
                      <BrokerIcon brokerName={masterAccount.brokerName} size="sm" />
                      <div className="flex-1 min-w-0">
                        <div className="font-medium text-xs text-gray-900 dark:text-gray-100 truncate">
                          {masterAccount.brokerName}
                        </div>
                        {masterAccount.accountNumber && (
                          <div className="text-xs text-gray-600 dark:text-gray-400">
                            {masterAccount.accountNumber}
                          </div>
                        )}
                      </div>
                    </div>

                    {/* Arrow */}
                    <span className="text-gray-400 text-lg flex-shrink-0">→</span>

                    {/* Slave Account */}
                    <div className="flex items-center gap-2 flex-1">
                      <BrokerIcon brokerName={slaveAccount.brokerName} size="sm" />
                      <div className="flex-1 min-w-0">
                        <div className="font-medium text-xs text-gray-900 dark:text-gray-100 truncate">
                          {slaveAccount.brokerName}
                        </div>
                        {slaveAccount.accountNumber && (
                          <div className="text-xs text-gray-600 dark:text-gray-400">
                            {slaveAccount.accountNumber}
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Copy Settings Section */}
              <div className="space-y-4">
                <div className="space-y-1">
                  <h3 className="text-sm font-medium flex items-center gap-2">
                    <span className="text-lg">⚙️</span>
                    {content.copySettingsLabel.value}
                  </h3>
                </div>

                {/* Lot Multiplier */}
                <div>
                  <Label htmlFor="lot_multiplier">
                    {content.lotMultiplier.value}
                  </Label>
                  <Input
                    id="lot_multiplier"
                    type="number"
                    step="0.01"
                    min="0.01"
                    max="100"
                    value={formData.lot_multiplier}
                    onChange={(e) => setFormData({ ...formData, lot_multiplier: parseFloat(e.target.value) || 1.0 })}
                    required
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    {content.lotMultiplierDescription.value}
                  </p>
                </div>

                {/* Reverse Trade */}
                <div className="flex items-center space-x-2">
                  <Checkbox
                    id="reverse_trade"
                    checked={formData.reverse_trade}
                    onCheckedChange={(checked) =>
                      setFormData({ ...formData, reverse_trade: checked as boolean })
                    }
                  />
                  <Label htmlFor="reverse_trade" className="cursor-pointer">
                    {content.reverseTrade.value} - {content.reverseDescription.value}
                  </Label>
                </div>
              </div>
            </div>

            <DialogFooter className="mt-6 flex-shrink-0 pt-4 border-t">
              <div className="flex w-full justify-between items-center">
                <div>
                  <Button type="button" variant="destructive" onClick={() => setShowDeleteConfirm(true)}>
                    {content.delete.value}
                  </Button>
                </div>
                <div className="flex gap-2">
                  <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                    {content.cancel.value}
                  </Button>
                  <Button type="submit">
                    {content.save.value}
                  </Button>
                </div>
              </div>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{content.deleteConfirmTitle.value}</AlertDialogTitle>
            <AlertDialogDescription>
              {content.deleteConfirmDescription.value}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{content.cancel.value}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {content.delete.value}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
