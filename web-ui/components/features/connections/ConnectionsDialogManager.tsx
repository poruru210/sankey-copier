'use client';

import { useState, useCallback } from 'react';
import { useIntlayer } from 'next-intlayer';
import { useToast } from '@/hooks/use-toast';
import { CreateConnectionDialog } from '@/components/features/connections/CreateConnectionDialog';
import { EditConnectionDrawer } from '@/components/features/connections/EditConnectionDrawer';
import { MasterSettingsDrawer } from '@/components/features/connections/MasterSettingsDrawer';
import type { CopySettings, CreateSettingsRequest, EaConnection } from '@/types';

interface ConnectionsDialogManagerProps {
  connections: EaConnection[];
  settings: CopySettings[];
  onCreate: (data: CreateSettingsRequest) => Promise<void>;
  onUpdate: (id: number, data: CopySettings) => Promise<void>;
  onDelete: (id: number) => Promise<void>;
}

export function ConnectionsDialogManager({
  connections,
  settings,
  onCreate,
  onUpdate,
  onDelete,
}: ConnectionsDialogManagerProps) {
  const content = useIntlayer('connections-view');
  const { toast } = useToast();

  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [editingSettings, setEditingSettings] = useState<CopySettings | null>(null);
  const [masterSettingsOpen, setMasterSettingsOpen] = useState(false);
  const [editingMasterAccount, setEditingMasterAccount] = useState<string>('');

  const openCreateDialog = useCallback(() => {
    setCreateDialogOpen(true);
  }, []);

  const openEditDialog = useCallback((setting: CopySettings) => {
    setEditingSettings(setting);
    setEditDialogOpen(true);
  }, []);

  const openMasterSettings = useCallback((masterAccount: string) => {
    setEditingMasterAccount(masterAccount);
    setMasterSettingsOpen(true);
  }, []);

  const handleCreateConnection = useCallback(
    async (data: CreateSettingsRequest) => {
      try {
        await onCreate(data);
      } catch (error) {
        toast({
          title: content.createFailed,
          description: error instanceof Error ? error.message : content.unknownError,
          variant: 'destructive',
        });
      }
    },
    [onCreate, toast, content.createFailed, content.unknownError]
  );

  const handleUpdateSettings = useCallback(
    async (data: CopySettings) => {
      try {
        await onUpdate(data.id, data);
        toast({
          title: content.settingsUpdated,
          description: `${data.master_account} → ${data.slave_account}`,
        });
      } catch (error) {
        toast({
          title: content.saveFailed,
          description: error instanceof Error ? error.message : content.unknownError,
          variant: 'destructive',
        });
      }
    },
    [onUpdate, toast, content.settingsUpdated, content.saveFailed, content.unknownError]
  );

  const handleDeleteSetting = useCallback(
    async (setting: CopySettings) => {
      try {
        await onDelete(setting.id);
      } catch (error) {
        toast({
          title: content.deleteFailed,
          description: error instanceof Error ? error.message : content.unknownError,
          variant: 'destructive',
        });
      }
    },
    [onDelete, toast, content.deleteFailed, content.unknownError]
  );

  return {
    openCreateDialog,
    openEditDialog,
    openMasterSettings,
    handleDeleteSetting, // Exposed for direct use if needed, though usually wired through Dialogs
    renderDialogs: (
      <>
        <CreateConnectionDialog
          open={createDialogOpen}
          onOpenChange={setCreateDialogOpen}
          onCreate={handleCreateConnection}
          connections={connections}
          existingSettings={settings}
        />

        {editingSettings && (
          <EditConnectionDrawer
            open={editDialogOpen}
            onOpenChange={setEditDialogOpen}
            onSave={handleUpdateSettings}
            onDelete={handleDeleteSetting}
            setting={editingSettings}
            connection={connections.find(c => c.account_id === editingSettings.slave_account)}
          />
        )}

        <MasterSettingsDrawer
          open={masterSettingsOpen}
          onOpenChange={setMasterSettingsOpen}
          masterAccount={editingMasterAccount}
          connection={connections.find(c => c.account_id === editingMasterAccount)}
        />
      </>
    ),
  };
}
