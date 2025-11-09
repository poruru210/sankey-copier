import { useState, useEffect, useCallback } from 'react';
import type { CopySettings, EaConnection, CreateSettingsRequest } from '@/types';

export function useSankeyCopier() {
  const [settings, setSettings] = useState<CopySettings[]>([]);
  const [connections, setConnections] = useState<EaConnection[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [wsMessages, setWsMessages] = useState<string[]>([]);

  // Fetch connections
  const fetchConnections = useCallback(async () => {
    try {
      const response = await fetch('/api/connections');
      if (!response.ok) {
        throw new Error(`Server returned ${response.status}: ${response.statusText}`);
      }
      const data = await response.json();
      if (data.success) {
        setConnections(data.data || []);
      }
    } catch (err) {
      if (err instanceof TypeError && err.message.includes('fetch')) {
        console.error('Cannot connect to server - is rust-server running?');
      } else {
        console.error('Failed to fetch connections:', err);
      }
    }
  }, []);

  // Fetch settings
  const fetchSettings = useCallback(async () => {
    try {
      setLoading(true);
      const response = await fetch('/api/settings');
      if (!response.ok) {
        throw new Error(`Server returned ${response.status}: ${response.statusText}`);
      }
      const data = await response.json();
      if (data.success) {
        setSettings(data.data || []);
        setError(null);
      } else {
        setError(data.error || 'Failed to load settings');
      }
    } catch (err) {
      if (err instanceof TypeError && (err.message.includes('fetch') || err.message.includes('Failed to fetch'))) {
        setError('Cannot connect to server. Please check if Rust Server is running.');
      } else if (err instanceof Error && err.message.includes('JSON')) {
        setError('Invalid server response. Rust Server may not be running correctly.');
      } else if (err instanceof Error && (err.message.includes('500') || err.message.includes('502') || err.message.includes('503'))) {
        setError('Cannot connect to server. Please check if Rust Server is running. (Proxy error)');
      } else {
        setError(err instanceof Error ? `Communication error: ${err.message}` : 'Unknown error');
      }
      console.error('Failed to fetch settings:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  // WebSocket connection
  useEffect(() => {
    const ws = new WebSocket(`ws://${window.location.host}/ws`);
    let isCleanup = false;

    ws.onopen = () => {
      if (!isCleanup) {
        console.log('WebSocket connected');
      }
    };

    ws.onmessage = (event) => {
      if (isCleanup) return;
      const message = event.data;
      console.log('WS message:', message);
      setWsMessages((prev) => [message, ...prev].slice(0, 20));

      if (message.startsWith('settings_')) {
        fetchSettings();
      }
    };

    ws.onerror = (error) => {
      if (!isCleanup && ws.readyState !== WebSocket.CLOSING && ws.readyState !== WebSocket.CLOSED) {
        console.error('WebSocket error:', error);
      }
    };

    ws.onclose = () => {
      if (!isCleanup) {
        console.log('WebSocket disconnected');
      }
    };

    return () => {
      isCleanup = true;
      if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
        ws.close();
      }
    };
  }, [fetchSettings]);

  // Initial load and periodic connection refresh
  useEffect(() => {
    fetchSettings();
    fetchConnections();
    const interval = setInterval(fetchConnections, 5000);
    return () => clearInterval(interval);
  }, [fetchSettings, fetchConnections]);

  // Toggle enabled status
  const toggleEnabled = async (id: number, currentStatus: boolean) => {
    try {
      const response = await fetch(`/api/settings/${id}/toggle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled: !currentStatus }),
      });
      const data = await response.json();
      if (data.success) {
        fetchSettings();
      } else {
        alert('Failed to toggle: ' + data.error);
      }
    } catch (err) {
      alert('Error: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  // Create new setting
  const createSetting = async (formData: CreateSettingsRequest) => {
    try {
      const response = await fetch('/api/settings', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(formData),
      });
      const data = await response.json();
      if (data.success) {
        fetchSettings();
      } else {
        alert('Failed to create: ' + data.error);
      }
    } catch (err) {
      alert('Error: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  // Update setting
  const updateSetting = async (id: number, updatedData: CopySettings) => {
    try {
      const response = await fetch(`/api/settings/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updatedData),
      });
      const data = await response.json();
      if (data.success) {
        fetchSettings();
      } else {
        alert('Failed to update: ' + data.error);
      }
    } catch (err) {
      alert('Error: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  // Delete setting
  const deleteSetting = async (id: number) => {
    if (!confirm('Are you sure you want to delete this connection?')) {
      return;
    }
    try {
      const response = await fetch(`/api/settings/${id}`, {
        method: 'DELETE',
      });
      const data = await response.json();
      if (data.success) {
        fetchSettings();
      } else {
        alert('Failed to delete: ' + data.error);
      }
    } catch (err) {
      alert('Error: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  return {
    settings,
    connections,
    loading,
    error,
    wsMessages,
    toggleEnabled,
    createSetting,
    updateSetting,
    deleteSetting,
  };
}
