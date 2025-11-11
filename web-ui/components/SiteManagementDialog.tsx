'use client';

import { useState } from 'react';
import { Plus, Edit, Trash2, Server } from 'lucide-react';
import { useSiteContext } from '@/lib/contexts/site-context';
import { Button } from './ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from './ui/dialog';
import { Input } from './ui/input';
import { Label } from './ui/label';

interface SiteFormData {
  name: string;
  siteUrl: string;
}

export function SiteManagementDialog() {
  const { sites, addSite, updateSite, deleteSite } = useSiteContext();
  const [isOpen, setIsOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [formData, setFormData] = useState<SiteFormData>({ name: '', siteUrl: '' });
  const [isFormVisible, setIsFormVisible] = useState(false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!formData.name.trim() || !formData.siteUrl.trim()) {
      return;
    }

    if (editingId) {
      updateSite(editingId, formData);
    } else {
      addSite(formData.name.trim(), formData.siteUrl.trim());
    }

    // Reset form
    setFormData({ name: '', siteUrl: '' });
    setEditingId(null);
    setIsFormVisible(false);
  };

  const handleEdit = (siteId: string) => {
    const site = sites.find(s => s.id === siteId);
    if (site) {
      setFormData({ name: site.name, siteUrl: site.siteUrl });
      setEditingId(siteId);
      setIsFormVisible(true);
    }
  };

  const handleDelete = (siteId: string) => {
    if (sites.length <= 1) {
      alert('最低1つのサイトが必要です');
      return;
    }

    const site = sites.find(s => s.id === siteId);
    if (site && confirm(`サイト "${site.name}" を削除してもよろしいですか？`)) {
      deleteSite(siteId);
    }
  };

  const handleCancel = () => {
    setFormData({ name: '', siteUrl: '' });
    setEditingId(null);
    setIsFormVisible(false);
  };

  const handleAddNew = () => {
    setFormData({ name: '', siteUrl: 'http://localhost:3000' });
    setEditingId(null);
    setIsFormVisible(true);
  };

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger asChild>
        <Button variant="ghost" size="sm" className="h-9 px-2">
          <Server className="h-4 w-4" />
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>サイト管理</DialogTitle>
          <DialogDescription>
            接続先サイトの追加、編集、削除を行います
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Site List */}
          <div className="space-y-2">
            <h3 className="text-sm font-medium">登録済みサイト</h3>
            <div className="space-y-2 max-h-[300px] overflow-y-auto">
              {sites.map((site) => (
                <div
                  key={site.id}
                  className="flex items-center justify-between gap-2 rounded-lg border p-3"
                >
                  <div className="flex-1 min-w-0">
                    <div className="font-medium truncate">{site.name}</div>
                    <div className="text-sm text-muted-foreground truncate">
                      {site.siteUrl}
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleEdit(site.id)}
                      className="h-8 px-2"
                    >
                      <Edit className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDelete(site.id)}
                      className="h-8 px-2 text-destructive hover:text-destructive"
                      disabled={sites.length <= 1}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Add/Edit Form */}
          {isFormVisible ? (
            <form onSubmit={handleSubmit} className="space-y-4 border-t pt-4">
              <h3 className="text-sm font-medium">
                {editingId ? 'サイトを編集' : '新しいサイトを追加'}
              </h3>
              <div className="space-y-2">
                <Label htmlFor="site-name">サイト名</Label>
                <Input
                  id="site-name"
                  type="text"
                  placeholder="例: Production Server"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="site-url">サイトURL</Label>
                <Input
                  id="site-url"
                  type="url"
                  placeholder="http://localhost:3000"
                  value={formData.siteUrl}
                  onChange={(e) => setFormData({ ...formData, siteUrl: e.target.value })}
                  required
                />
              </div>
              <div className="flex gap-2">
                <Button type="submit" size="sm">
                  {editingId ? '更新' : '追加'}
                </Button>
                <Button type="button" variant="outline" size="sm" onClick={handleCancel}>
                  キャンセル
                </Button>
              </div>
            </form>
          ) : (
            <Button
              onClick={handleAddNew}
              variant="outline"
              size="sm"
              className="w-full"
            >
              <Plus className="h-4 w-4 mr-2" />
              新しいサイトを追加
            </Button>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
