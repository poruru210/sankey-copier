import { useState, useCallback, useEffect } from 'react';
import { useAtom } from 'jotai';
import type { CopySettings } from '@/types';
import {
  hoveredSourceIdAtom,
  hoveredReceiverIdAtom,
  selectedSourceIdAtom,
} from '@/lib/atoms/ui';

interface UseConnectionHighlightReturn {
  hoveredSourceId: string | null;
  hoveredReceiverId: string | null;
  selectedSourceId: string | null;
  setHoveredSource: (id: string | null) => void;
  setHoveredReceiver: (id: string | null) => void;
  handleSourceTap: (id: string | null) => void;
  clearSelection: () => void;
  isAccountHighlighted: (accountId: string, type: 'source' | 'receiver') => boolean;
  isMobile: boolean;
  getConnectedReceivers: (sourceId: string) => string[];
  getConnectedSources: (receiverId: string) => string[];
}

/**
 * Custom hook for managing hover states and connection highlighting
 *
 * @param settings - List of copy settings to determine connections
 * @returns Hover state management and helper functions
 */
export function useConnectionHighlight(
  settings: CopySettings[]
): UseConnectionHighlightReturn {
  const [hoveredSourceId, setHoveredSourceId] = useAtom(hoveredSourceIdAtom);
  const [hoveredReceiverId, setHoveredReceiverId] = useAtom(hoveredReceiverIdAtom);
  const [selectedSourceId, setSelectedSourceId] = useAtom(selectedSourceIdAtom);
  const [isMobile, setIsMobile] = useState(false);

  // Detect mobile viewport
  useEffect(() => {
    const checkMobile = () => {
      setIsMobile(window.innerWidth < 768);
    };

    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  // Memoize connection mappings
  const getConnectedReceivers = useCallback(
    (sourceId: string): string[] => {
      return settings
        .filter((s) => s.master_account === sourceId)
        .map((s) => s.slave_account);
    },
    [settings]
  );

  const getConnectedSources = useCallback(
    (receiverId: string): string[] => {
      return settings
        .filter((s) => s.slave_account === receiverId)
        .map((s) => s.master_account);
    },
    [settings]
  );

  // Determine if an account should be highlighted
  const isAccountHighlighted = useCallback(
    (accountId: string, type: 'source' | 'receiver'): boolean => {
      // On mobile, use selected state instead of hover
      const activeSourceId = isMobile ? selectedSourceId : hoveredSourceId;
      const activeReceiverId = isMobile ? null : hoveredReceiverId;

      if (type === 'source') {
        // Highlight if this source is active (hovered or selected)
        if (activeSourceId === accountId) return true;

        // Highlight if a connected receiver is hovered (desktop only)
        if (activeReceiverId) {
          const connectedReceivers = getConnectedReceivers(accountId);
          return connectedReceivers.includes(activeReceiverId);
        }
      } else {
        // type === 'receiver'
        // Highlight if this receiver is hovered (desktop only)
        if (activeReceiverId === accountId) return true;

        // Highlight if a connected source is active
        if (activeSourceId) {
          const connectedSources = getConnectedSources(accountId);
          return connectedSources.includes(activeSourceId);
        }
      }

      return false;
    },
    [
      isMobile,
      selectedSourceId,
      hoveredSourceId,
      hoveredReceiverId,
      getConnectedReceivers,
      getConnectedSources,
    ]
  );

  // Stable setter functions
  const setHoveredSource = useCallback((id: string | null) => {
    setHoveredSourceId(id);
  }, [setHoveredSourceId]);

  const setHoveredReceiver = useCallback((id: string | null) => {
    setHoveredReceiverId(id);
  }, [setHoveredReceiverId]);

  // Handle source selection on mobile (from dropdown)
  const handleSourceTap = useCallback(
    (id: string | null) => {
      if (isMobile) {
        // Set selected source or clear if null/empty
        setSelectedSourceId(id);
      }
    },
    [isMobile, setSelectedSourceId]
  );

  // Clear selection
  const clearSelection = useCallback(() => {
    setSelectedSourceId(null);
  }, [setSelectedSourceId]);

  return {
    hoveredSourceId,
    hoveredReceiverId,
    selectedSourceId,
    setHoveredSource,
    setHoveredReceiver,
    handleSourceTap,
    clearSelection,
    isAccountHighlighted,
    isMobile,
    getConnectedReceivers,
    getConnectedSources,
  };
}
