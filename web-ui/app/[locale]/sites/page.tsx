'use client';

import { useState, useEffect } from 'react';
import { useIntlayer } from 'next-intlayer';
import { Header } from '@/components/Header';
import { ParticlesBackground } from '@/components/ParticlesBackground';
import { useSiteContext } from '@/lib/contexts/site-context';
import { useSidebar } from '@/lib/contexts/sidebar-context';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { AlertCircle, Trash2, Plus, Edit2 } from 'lucide-react';
import { Site } from '@/lib/types/site';
import { cn } from '@/lib/utils';

// Sites management page
// Allows users to add, edit, and delete SANKEY Copier server connections
export default function SitesPage() {
  const content = useIntlayer('sites-page');
  const { sites, addSite, updateSite, deleteSite, selectedSiteId, selectSite } = useSiteContext();
  const { isOpen: isSidebarOpen, isMobile, serverLogHeight } = useSidebar();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [formData, setFormData] = useState({ name: '', siteUrl: '' });
  const [error, setError] = useState<string>('');

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
      setError(content.errorSiteNameRequired.value);
      return;
    }
    if (!formData.siteUrl.trim()) {
      setError(content.errorSiteUrlRequired.value);
      return;
    }

    // Validate URL format
    try {
      new URL(formData.siteUrl);
    } catch {
      setError(content.errorInvalidUrl.value);
      return;
    }

    if (editingId && editingId !== 'new') {
      // Update existing site
      updateSite(editingId, {
        name: formData.name.trim(),
        siteUrl: formData.siteUrl.trim(),
      });
    } else {
      // Add new site
      const newSite = addSite(formData.name.trim(), formData.siteUrl.trim());
      // Automatically select the newly created site
      selectSite(newSite.id);
    }

    // Reset form
    handleCancelEdit();
  };

  const handleDelete = (site: Site) => {
    if (sites.length === 1) {
      setError(content.errorCannotDeleteLast.value);
      return;
    }

    const confirmMessage = content.confirmDelete.value.replace('{siteName}', site.name);
    if (window.confirm(confirmMessage)) {
      deleteSite(site.id);
      setError('');
    }
  };

  const handleStartAdd = () => {
    setEditingId('new');
    setFormData({ name: '', siteUrl: '' });
    setError('');
  };

  return (
    <div className="h-screen bg-background relative overflow-hidden flex flex-col">
      {/* Particles Background */}
      <ParticlesBackground />

      {/* Main Content */}
      <div className="relative z-10 flex flex-col h-full">
        <Header />
        <div
          className={cn(
            'overflow-y-auto transition-all duration-300',
            !isMobile && (isSidebarOpen ? 'lg:ml-64' : 'lg:ml-16')
          )}
          style={{
            height: `calc(100vh - 56px - ${serverLogHeight}px)`,
            maxHeight: `calc(100vh - 56px - ${serverLogHeight}px)`
          }}
        >
          <div className="container mx-auto p-6 max-w-[1200px]">
          {/* Page Title */}
          <div className="mb-6">
            <h1 className="text-3xl font-bold mb-2">{content.title}</h1>
            <p className="text-muted-foreground">{content.description}</p>
          </div>

          {/* Error Message */}
          {error && (
            <div className="rounded-md bg-red-50 dark:bg-red-950 p-3 border border-red-200 dark:border-red-800 mb-6">
              <div className="flex items-start">
                <AlertCircle className="h-5 w-5 text-red-400 mr-2 flex-shrink-0" />
                <p className="text-sm text-red-800 dark:text-red-200">{error}</p>
              </div>
            </div>
          )}

          {/* Site List */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold">{content.registeredSites}</h3>
              {editingId !== 'new' && (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={handleStartAdd}
                  className="gap-2"
                >
                  <Plus className="h-4 w-4" />
                  {content.addButton}
                </Button>
              )}
            </div>

            <div className="space-y-3">
              {sites.map((site) => (
                <div
                  key={site.id}
                  className={`p-4 rounded-lg border transition-colors ${
                    site.id === selectedSiteId
                      ? 'border-blue-500 bg-blue-50 dark:bg-blue-950'
                      : 'border-gray-200 dark:border-gray-700 bg-card hover:bg-accent/50'
                  }`}
                >
                  {editingId === site.id ? (
                    // Edit Mode
                    <div className="space-y-3">
                      <div>
                        <Label htmlFor={`name-${site.id}`} className="text-sm">
                          {content.siteName}
                        </Label>
                        <Input
                          id={`name-${site.id}`}
                          value={formData.name}
                          onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                          placeholder={content.siteNamePlaceholder.value}
                          className="mt-1"
                        />
                      </div>
                      <div>
                        <Label htmlFor={`url-${site.id}`} className="text-sm">
                          {content.siteUrl}
                        </Label>
                        <Input
                          id={`url-${site.id}`}
                          value={formData.siteUrl}
                          onChange={(e) => setFormData({ ...formData, siteUrl: e.target.value })}
                          placeholder={content.siteUrlPlaceholder.value}
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
                          {content.cancel}
                        </Button>
                        <Button type="button" size="sm" onClick={handleSave}>
                          {content.save}
                        </Button>
                      </div>
                    </div>
                  ) : (
                    // Display Mode
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <h4 className="font-semibold text-base">{site.name}</h4>
                          {site.id === selectedSiteId && (
                            <span className="text-xs px-2 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 font-medium">
                              {content.selected}
                            </span>
                          )}
                        </div>
                        <p className="text-sm text-muted-foreground mt-1 truncate">
                          {site.siteUrl}
                        </p>
                      </div>
                      <div className="flex gap-1 flex-shrink-0">
                        <Button
                          type="button"
                          size="sm"
                          variant="ghost"
                          onClick={() => handleStartEdit(site)}
                          className="h-9 w-9 p-0"
                          title={content.edit.value}
                        >
                          <Edit2 className="h-4 w-4" />
                        </Button>
                        <Button
                          type="button"
                          size="sm"
                          variant="ghost"
                          onClick={() => handleDelete(site)}
                          disabled={sites.length === 1}
                          className="h-9 w-9 p-0 text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-950 disabled:opacity-50"
                          title={content.delete.value}
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
                <div className="p-4 rounded-lg border border-green-500 bg-green-50 dark:bg-green-950">
                  <div className="space-y-3">
                    <h4 className="font-semibold text-base">{content.addNewSite}</h4>
                    <div>
                      <Label htmlFor="name-new" className="text-sm">
                        {content.siteName}
                      </Label>
                      <Input
                        id="name-new"
                        value={formData.name}
                        onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                        placeholder={content.siteNamePlaceholder.value}
                        className="mt-1"
                      />
                    </div>
                    <div>
                      <Label htmlFor="url-new" className="text-sm">
                        {content.siteUrl}
                      </Label>
                      <Input
                        id="url-new"
                        value={formData.siteUrl}
                        onChange={(e) => setFormData({ ...formData, siteUrl: e.target.value })}
                        placeholder={content.siteUrlPlaceholder.value}
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
                        {content.cancel}
                      </Button>
                      <Button type="button" size="sm" onClick={handleSave}>
                        {content.add}
                      </Button>
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Info Message */}
            <div className="p-4 bg-blue-50 dark:bg-blue-950 rounded-lg border border-blue-200 dark:border-blue-800">
              <p className="text-sm text-blue-800 dark:text-blue-200">
                {content.infoMessage}
              </p>
            </div>
          </div>
          </div>
        </div>
      </div>
    </div>
  );
}
