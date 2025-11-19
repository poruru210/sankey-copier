'use client';

import { useState, useEffect } from 'react';
import { useAtom } from 'jotai';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { AlertCircle, Trash2, Plus, Edit2 } from 'lucide-react';
import { sitesAtom, selectedSiteIdAtom } from '@/lib/atoms/site';
import { Site, DEFAULT_SITE } from '@/lib/types/site';

interface SiteManagementDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SiteManagementDialog({ open, onOpenChange }: SiteManagementDialogProps) {
  const [sites, setSites] = useAtom(sitesAtom);
  const [selectedSiteId, setSelectedSiteId] = useAtom(selectedSiteIdAtom);

  const [editingId, setEditingId] = useState<string | null>(null);
  const [formData, setFormData] = useState({ name: '', siteUrl: '' });
  const [error, setError] = useState<string>('');

  // Reset form when dialog closes
  useEffect(() => {
    if (!open) {
      setEditingId(null);
      setFormData({ name: '', siteUrl: '' });
      setError('');
    }
  }, [open]);

  const handleStartEdit = (site: Site) => {
    setEditingId(site.id);
    setFormData({ name: site.name, siteUrl: site.siteUrl });
    setError('');
  };

  const handleCancelEdit = () => {
    setEditingId(null);
    setFormData({ name: '', siteUrl: '' });
    setError('');
  };

  const handleSave = () => {
    // Validate
    if (!formData.name.trim()) {
      setError('サイト名を入力してください');
      return;
    }
    if (!formData.siteUrl.trim()) {
      setError('URLを入力してください');
      return;
    }

    // Validate URL format
    try {
      new URL(formData.siteUrl);
    } catch {
      setError('有効なURLを入力してください（例: http://localhost:3000）');
      return;
    }

    if (editingId && editingId !== 'new') {
      // Update existing site
      setSites((prev) =>
        prev.map((site) =>
          site.id === editingId
            ? { ...site, name: formData.name.trim(), siteUrl: formData.siteUrl.trim() }
            : site
        )
      );
    } else {
      // Add new site
      const newSite: Site = {
        id: crypto.randomUUID(),
        name: formData.name.trim(),
        siteUrl: formData.siteUrl.trim(),
      };
      setSites((prev) => [...prev, newSite]);
      // Automatically select the newly created site
      setSelectedSiteId(newSite.id);
    }

    // Reset form
    handleCancelEdit();
  };

  const handleDelete = (site: Site) => {
    if (sites.length === 1) {
      setError('最後のサイトは削除できません');
      return;
    }

    if (window.confirm(`「${site.name}」を削除しますか？`)) {
      setSites((prev) => prev.filter((s) => s.id !== site.id));

      // If deleted site was selected, select the first available site
      if (selectedSiteId === site.id) {
        const remainingSites = sites.filter((s) => s.id !== site.id);
        if (remainingSites.length > 0) {
          setSelectedSiteId(remainingSites[0].id);
        }
      }
      setError('');
    }
  };

  const handleStartAdd = () => {
    setEditingId('new');
    setFormData({ name: '', siteUrl: '' });
    setError('');
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>サイト管理</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* Error Message */}
          {error && (
            <div className="rounded-md bg-red-50 dark:bg-red-950 p-3 border border-red-200 dark:border-red-800">
              <div className="flex items-start">
                <AlertCircle className="h-5 w-5 text-red-400 mr-2 flex-shrink-0" />
                <p className="text-sm text-red-800 dark:text-red-200">{error}</p>
              </div>
            </div>
          )}

          {/* Site List */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-medium">登録済みサイト</h3>
              {editingId !== 'new' && (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={handleStartAdd}
                  className="gap-2"
                >
                  <Plus className="h-4 w-4" />
                  追加
                </Button>
              )}
            </div>

            <div className="space-y-2">
              {sites.map((site) => (
                <div
                  key={site.id}
                  className={`p-3 rounded-lg border ${site.id === selectedSiteId
                      ? 'border-blue-500 bg-blue-50 dark:bg-blue-950'
                      : 'border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800'
                    }`}
                >
                  {editingId === site.id ? (
                    // Edit Mode
                    <div className="space-y-3">
                      <div>
                        <Label htmlFor={`name-${site.id}`} className="text-xs">
                          サイト名
                        </Label>
                        <Input
                          id={`name-${site.id}`}
                          value={formData.name}
                          onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                          placeholder="例: Local"
                          className="mt-1"
                        />
                      </div>
                      <div>
                        <Label htmlFor={`url-${site.id}`} className="text-xs">
                          URL
                        </Label>
                        <Input
                          id={`url-${site.id}`}
                          value={formData.siteUrl}
                          onChange={(e) => setFormData({ ...formData, siteUrl: e.target.value })}
                          placeholder="例: http://localhost:3000"
                          className="mt-1"
                        />
                      </div>
                      <div className="flex gap-2 justify-end">
                        <Button
                          type="button"
                          size="sm"
                          variant="outline"
                          onClick={handleCancelEdit}
                        >
                          キャンセル
                        </Button>
                        <Button type="button" size="sm" onClick={handleSave}>
                          保存
                        </Button>
                      </div>
                    </div>
                  ) : (
                    // Display Mode
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <h4 className="font-medium text-sm">{site.name}</h4>
                          {site.id === selectedSiteId && (
                            <span className="text-xs px-2 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300">
                              選択中
                            </span>
                          )}
                        </div>
                        <p className="text-xs text-muted-foreground mt-1 truncate">
                          {site.siteUrl}
                        </p>
                      </div>
                      <div className="flex gap-1 flex-shrink-0">
                        <Button
                          type="button"
                          size="sm"
                          variant="ghost"
                          onClick={() => handleStartEdit(site)}
                          className="h-8 w-8 p-0"
                        >
                          <Edit2 className="h-4 w-4" />
                        </Button>
                        <Button
                          type="button"
                          size="sm"
                          variant="ghost"
                          onClick={() => handleDelete(site)}
                          disabled={sites.length === 1}
                          className="h-8 w-8 p-0 text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-950"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              ))}

              {/* Add New Site Form */}
              {editingId === 'new' && (
                <div className="p-3 rounded-lg border border-green-500 bg-green-50 dark:bg-green-950">
                  <div className="space-y-3">
                    <h4 className="font-medium text-sm">新しいサイトを追加</h4>
                    <div>
                      <Label htmlFor="name-new" className="text-xs">
                        サイト名
                      </Label>
                      <Input
                        id="name-new"
                        value={formData.name}
                        onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                        placeholder="例: Remote Server"
                        className="mt-1"
                      />
                    </div>
                    <div>
                      <Label htmlFor="url-new" className="text-xs">
                        URL
                      </Label>
                      <Input
                        id="url-new"
                        value={formData.siteUrl}
                        onChange={(e) => setFormData({ ...formData, siteUrl: e.target.value })}
                        placeholder="例: http://192.168.1.100:3000"
                        className="mt-1"
                      />
                    </div>
                    <div className="flex gap-2 justify-end">
                      <Button
                        type="button"
                        size="sm"
                        variant="outline"
                        onClick={handleCancelEdit}
                      >
                        キャンセル
                      </Button>
                      <Button type="button" size="sm" onClick={handleSave}>
                        追加
                      </Button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Info Message */}
          <div className="p-3 bg-blue-50 dark:bg-blue-950 rounded-lg border border-blue-200 dark:border-blue-800">
            <p className="text-xs text-blue-800 dark:text-blue-200">
              💡 複数のSANKEY Copierサーバーを登録して切り替えることができます。
              設定はブラウザのlocalStorageに保存されます。
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button type="button" onClick={() => onOpenChange(false)}>
            閉じる
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
