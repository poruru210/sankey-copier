'use client';

import { useState, useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import type { CopySettings, CreateSettingsRequest } from '@/types';

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (data: CreateSettingsRequest | CopySettings) => void;
  initialData?: CopySettings | null;
}

export function SettingsDialog({ open, onOpenChange, onSave, initialData }: SettingsDialogProps) {
  const content = useIntlayer('settings-dialog');
  const [formData, setFormData] = useState({
    master_account: '',
    slave_account: '',
    lot_multiplier: 1.0,
    reverse_trade: false,
  });

  useEffect(() => {
    if (initialData) {
      setFormData({
        master_account: initialData.master_account,
        slave_account: initialData.slave_account,
        lot_multiplier: initialData.lot_multiplier || 1.0,
        reverse_trade: initialData.reverse_trade,
      });
    } else {
      setFormData({
        master_account: '',
        slave_account: '',
        lot_multiplier: 1.0,
        reverse_trade: false,
      });
    }
  }, [initialData, open]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (initialData) {
      onSave({
        ...initialData,
        ...formData,
      });
    } else {
      onSave(formData);
    }
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{initialData ? content.editTitle : content.createTitle}</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="space-y-4">
            <div>
              <Label htmlFor="master_account">{content.masterAccount}</Label>
              <Input
                id="master_account"
                value={formData.master_account}
                onChange={(e) => setFormData({ ...formData, master_account: e.target.value })}
                placeholder="MASTER_001"
                required
              />
            </div>
            <div>
              <Label htmlFor="slave_account">{content.slaveAccount}</Label>
              <Input
                id="slave_account"
                value={formData.slave_account}
                onChange={(e) => setFormData({ ...formData, slave_account: e.target.value })}
                placeholder="SLAVE_001"
                required
              />
            </div>
            <div>
              <Label htmlFor="lot_multiplier">{content.lotMultiplier}</Label>
              <Input
                id="lot_multiplier"
                type="number"
                step="0.01"
                value={formData.lot_multiplier}
                onChange={(e) => setFormData({ ...formData, lot_multiplier: parseFloat(e.target.value) })}
                required
              />
            </div>
            <div className="flex items-center space-x-2">
              <Checkbox
                id="reverse_trade"
                checked={formData.reverse_trade}
                onChange={(e) => setFormData({ ...formData, reverse_trade: (e.target as HTMLInputElement).checked })}
              />
              <Label htmlFor="reverse_trade" className="cursor-pointer">
                {content.reverseTrade} - {content.reverseDescription}
              </Label>
            </div>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              {content.cancel}
            </Button>
            <Button type="submit">{initialData ? content.save : content.create}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
