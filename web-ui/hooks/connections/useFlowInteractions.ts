import { useCallback, useRef } from 'react';
import { NodeChange, Node } from '@xyflow/react';
import { useSetAtom } from 'jotai';
import {
  hoveredSourceIdAtom,
  hoveredReceiverIdAtom,
} from '@/lib/atoms/ui';

interface UseFlowInteractionsProps {
  isMobile: boolean;
  onNodesChange: (changes: NodeChange[]) => void;
}

export function useFlowInteractions({ isMobile, onNodesChange }: UseFlowInteractionsProps) {
  const setHoveredSource = useSetAtom(hoveredSourceIdAtom);
  const setHoveredReceiver = useSetAtom(hoveredReceiverIdAtom);

  // Track user-dragged nodes to preserve their positions
  const userDraggedNodesRef = useRef<Set<string>>(new Set());

  // Track user drags
  const onNodesChangeWithTracking = useCallback(
    (changes: NodeChange[]) => {
      changes.forEach((change) => {
        if (change.type === 'position' && change.dragging && 'id' in change) {
          userDraggedNodesRef.current.add(change.id);
        }
      });
      onNodesChange(changes);
    },
    [onNodesChange]
  );

  // Handle node hover
  const onNodeMouseEnter = useCallback(
    (event: React.MouseEvent, node: Node) => {
      if (!isMobile) {
        if (node.id.startsWith('source-')) {
          const accountId = node.id.replace('source-', '');
          setHoveredSource(accountId);
        } else if (node.id.startsWith('receiver-')) {
          const accountId = node.id.replace('receiver-', '');
          setHoveredReceiver(accountId);
        }
      }
    },
    [isMobile, setHoveredSource, setHoveredReceiver]
  );

  const onNodeMouseLeave = useCallback(() => {
    if (!isMobile) {
      setHoveredSource(null);
      setHoveredReceiver(null);
    }
  }, [isMobile, setHoveredSource, setHoveredReceiver]);

  return {
    onNodesChangeWithTracking,
    onNodeMouseEnter,
    onNodeMouseLeave,
    userDraggedNodesRef,
  };
}
