import { useState, useCallback } from 'react';
import type { MtInstallation, MtInstallationsResponse, ApiResponse } from '@/types';
import { useApiClient } from '@/lib/contexts/site-context';

export function useMtInstallations() {
  const apiClient = useApiClient();
  const [installations, setInstallations] = useState<MtInstallation[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installing, setInstalling] = useState<string | null>(null); // ID of installation being installed

  // Fetch MT installations
  const fetchInstallations = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await apiClient.get<ApiResponse<MtInstallationsResponse>>('/mt-installations');

      if (data.success && data.data) {
        setInstallations(data.data.data || []);
      } else {
        setError(data.error || 'Failed to load MT installations');
      }
    } catch (err) {
      if (err instanceof TypeError && (err.message.includes('fetch') || err.message.includes('Failed to fetch'))) {
        setError('Cannot connect to server. Please check if Rust Server is running.');
      } else if (err instanceof Error && err.message.includes('JSON')) {
        setError('Invalid server response. Rust Server may not be running correctly.');
      } else {
        setError(err instanceof Error ? `Communication error: ${err.message}` : 'Unknown error');
      }
      console.error('Failed to fetch MT installations:', err);
    } finally {
      setLoading(false);
    }
  }, [apiClient]);

  // Install components to MT installation
  const installToMt = async (id: string): Promise<{ success: boolean; message?: string }> => {
    try {
      setInstalling(id);
      const data = await apiClient.post<ApiResponse<string>>(`/mt-installations/${id}/install`);

      if (data.success) {
        // Refresh installations to get updated component status
        await fetchInstallations();
        return { success: true, message: data.data };
      } else {
        return { success: false, message: data.error || 'Installation failed' };
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      console.error('Failed to install to MT:', err);
      return { success: false, message: `Error: ${message}` };
    } finally {
      setInstalling(null);
    }
  };

  return {
    installations,
    loading,
    error,
    installing,
    fetchInstallations,
    installToMt,
  };
}
